const http = require('http');
const fs = require('fs');
const path = require('path');
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');

const PORT = Number(process.env.SG_UI_PORT || 4173);
const GRPC_ADDRESS = process.env.SG_GRPC_ADDR || '127.0.0.1:50051';
const DIST_DIR = path.resolve(__dirname, 'dist');
const PROTO_CANDIDATES = [
  path.resolve(__dirname, 'proto', 'sentinelguard.proto'),
  path.resolve(__dirname, '..', 'agent', 'proto', 'sentinelguard.proto'),
];

function resolveProtoPath() {
  for (const candidate of PROTO_CANDIDATES) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  throw new Error(`Missing proto file. Checked: ${PROTO_CANDIDATES.join(', ')}`);
}

function loadGrpcClient() {
  const protoPath = resolveProtoPath();
  const packageDefinition = protoLoader.loadSync(protoPath, {
    keepCase: false,
    longs: Number,
    enums: String,
    defaults: true,
    oneofs: true,
  });
  const protoDescriptor = grpc.loadPackageDefinition(packageDefinition);
  const SentinelGuardService = protoDescriptor.sentinelguard?.SentinelGuardService;
  if (!SentinelGuardService) {
    throw new Error('SentinelGuardService definition not found in loaded proto');
  }
  return new SentinelGuardService(GRPC_ADDRESS, grpc.credentials.createInsecure());
}

const grpcClient = loadGrpcClient();

function callUnary(method, request = {}) {
  return new Promise((resolve, reject) => {
    if (typeof grpcClient[method] !== 'function') {
      reject(new Error(`gRPC method not available: ${method}`));
      return;
    }

    grpcClient[method](request, (error, response) => {
      if (error) {
        reject(error);
        return;
      }
      resolve(response);
    });
  });
}

async function getComponentHealth() {
  try {
    const health = await callUnary('getSystemHealth', {});
    return {
      uiMode: 'browser',
      webBridgeRunning: true,
      grpcReachable: true,
      grpcAddress: GRPC_ADDRESS,
      agentRunning: Boolean(health.agentRunning),
      driverLoaded: Boolean(health.driverLoaded),
      lastError: null,
    };
  } catch (error) {
    return {
      uiMode: 'browser',
      webBridgeRunning: true,
      grpcReachable: false,
      grpcAddress: GRPC_ADDRESS,
      agentRunning: false,
      driverLoaded: false,
      lastError: error instanceof Error ? error.message : String(error),
    };
  }
}

async function getDashboardSnapshot() {
  const componentHealth = await getComponentHealth();

  if (!componentHealth.grpcReachable) {
    return {
      fetchedAt: Date.now(),
      componentHealth,
      systemHealth: null,
      processRiskOverview: { processes: [] },
      quarantinedProcesses: { processes: [] },
      detectorLogs: { entries: [] },
      errors: {
        systemHealth: componentHealth.lastError,
        processRiskOverview: componentHealth.lastError,
        quarantinedProcesses: componentHealth.lastError,
        detectorLogs: componentHealth.lastError,
      },
    };
  }

  const [systemHealth, processRiskOverview, quarantinedProcesses, detectorLogs] =
    await Promise.allSettled([
      callUnary('getSystemHealth', {}),
      callUnary('getProcessRiskOverview', { limit: 50 }),
      callUnary('getQuarantinedProcesses', {}),
      callUnary('getDetectorLogs', { sinceTimestamp: 0, limit: 100 }),
    ]);

  return {
    fetchedAt: Date.now(),
    componentHealth,
    systemHealth: systemHealth.status === 'fulfilled' ? systemHealth.value : null,
    processRiskOverview:
      processRiskOverview.status === 'fulfilled'
        ? processRiskOverview.value
        : { processes: [] },
    quarantinedProcesses:
      quarantinedProcesses.status === 'fulfilled'
        ? quarantinedProcesses.value
        : { processes: [] },
    detectorLogs:
      detectorLogs.status === 'fulfilled' ? detectorLogs.value : { entries: [] },
    errors: {
      systemHealth: systemHealth.status === 'rejected' ? systemHealth.reason.message : null,
      processRiskOverview:
        processRiskOverview.status === 'rejected' ? processRiskOverview.reason.message : null,
      quarantinedProcesses:
        quarantinedProcesses.status === 'rejected' ? quarantinedProcesses.reason.message : null,
      detectorLogs:
        detectorLogs.status === 'rejected' ? detectorLogs.reason.message : null,
    },
  };
}

function writeJson(res, statusCode, payload) {
  const body = JSON.stringify(payload);
  res.writeHead(statusCode, {
    'Content-Type': 'application/json; charset=utf-8',
    'Content-Length': Buffer.byteLength(body),
    'Cache-Control': 'no-store',
  });
  res.end(body);
}

function readRequestBody(req) {
  return new Promise((resolve, reject) => {
    let body = '';
    req.on('data', (chunk) => {
      body += chunk;
    });
    req.on('end', () => resolve(body));
    req.on('error', reject);
  });
}

async function handleApi(req, res, url) {
  try {
    if (req.method === 'GET' && url.pathname === '/api/component-health') {
      writeJson(res, 200, await getComponentHealth());
      return true;
    }

    if (req.method === 'GET' && url.pathname === '/api/dashboard') {
      writeJson(res, 200, await getDashboardSnapshot());
      return true;
    }

    if (req.method === 'GET' && url.pathname === '/api/system-health') {
      writeJson(res, 200, await callUnary('getSystemHealth', {}));
      return true;
    }

    if (req.method === 'GET' && url.pathname === '/api/process-risk-overview') {
      const limit = Number(url.searchParams.get('limit') || 100);
      writeJson(res, 200, await callUnary('getProcessRiskOverview', { limit }));
      return true;
    }

    if (req.method === 'GET' && url.pathname === '/api/quarantined-processes') {
      writeJson(res, 200, await callUnary('getQuarantinedProcesses', {}));
      return true;
    }

    if (req.method === 'GET' && url.pathname === '/api/detector-logs') {
      const sinceTimestamp = Number(url.searchParams.get('sinceTimestamp') || 0);
      const limit = Number(url.searchParams.get('limit') || 100);
      writeJson(
        res,
        200,
        await callUnary('getDetectorLogs', { sinceTimestamp, limit })
      );
      return true;
    }

    if (req.method === 'POST' && url.pathname === '/api/release-from-quarantine') {
      const rawBody = await readRequestBody(req);
      const body = rawBody ? JSON.parse(rawBody) : {};
      const processId = Number(body.processId);
      writeJson(
        res,
        200,
        await callUnary('releaseFromQuarantine', { processId })
      );
      return true;
    }

    if (req.method === 'GET' && url.pathname === '/api/alerts/stream') {
      const sinceTimestamp = Number(url.searchParams.get('sinceTimestamp') || 0);
      res.writeHead(200, {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache, no-transform',
        Connection: 'keep-alive',
      });

      const stream = grpcClient.getAlerts({ sinceTimestamp });
      const heartbeat = setInterval(() => {
        res.write(': ping\n\n');
      }, 15000);

      stream.on('data', (alert) => {
        res.write(`data: ${JSON.stringify(alert)}\n\n`);
      });

      stream.on('error', (error) => {
        const payload = JSON.stringify({
          __streamError: true,
          message: error.message,
        });
        res.write(`event: error\ndata: ${payload}\n\n`);
      });

      stream.on('end', () => {
        clearInterval(heartbeat);
        res.end();
      });

      req.on('close', () => {
        clearInterval(heartbeat);
        stream.cancel();
      });

      return true;
    }
  } catch (error) {
    writeJson(res, 502, {
      error: error instanceof Error ? error.message : 'Request failed',
    });
    return true;
  }

  return false;
}

function getContentType(filePath) {
  if (filePath.endsWith('.html')) return 'text/html; charset=utf-8';
  if (filePath.endsWith('.js')) return 'text/javascript; charset=utf-8';
  if (filePath.endsWith('.css')) return 'text/css; charset=utf-8';
  if (filePath.endsWith('.json')) return 'application/json; charset=utf-8';
  if (filePath.endsWith('.svg')) return 'image/svg+xml';
  if (filePath.endsWith('.png')) return 'image/png';
  return 'application/octet-stream';
}

function serveStatic(res, filePath) {
  if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
    return false;
  }

  res.writeHead(200, {
    'Content-Type': getContentType(filePath),
  });
  fs.createReadStream(filePath).pipe(res);
  return true;
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url || '/', `http://${req.headers.host || 'localhost'}`);

  if (await handleApi(req, res, url)) {
    return;
  }

  const requestedPath = url.pathname === '/' ? '/index.html' : url.pathname;
  const safePath = path.normalize(requestedPath).replace(/^(\.\.[/\\])+/, '');
  const assetPath = path.join(DIST_DIR, safePath);

  if (serveStatic(res, assetPath)) {
    return;
  }

  const spaEntry = path.join(DIST_DIR, 'index.html');
  if (serveStatic(res, spaEntry)) {
    return;
  }

  res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
  res.end('UI bundle not found. Build the UI with "npm run build:web".');
});

server.listen(PORT, () => {
  console.log(`SentinelGuard Web UI listening on http://localhost:${PORT}`);
  console.log(`Proxying SentinelGuard gRPC backend at ${GRPC_ADDRESS}`);
});
