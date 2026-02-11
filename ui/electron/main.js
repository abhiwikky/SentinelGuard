const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');

const GRPC_ADDRESS = process.env.SG_GRPC_ADDR || '127.0.0.1:50051';
const ALERT_STREAM_CHANNEL = 'sg:alerts:new';
const ALERT_HISTORY_LIMIT = 128;

let mainWindow = null;
let grpcClient = null;
let alertsStream = null;

function resolveProtoPath() {
  const candidates = [
    path.resolve(__dirname, '../../agent/proto/sentinelguard.proto'),
    path.resolve(process.cwd(), '../agent/proto/sentinelguard.proto'),
    path.resolve(process.resourcesPath || '.', 'agent/proto/sentinelguard.proto'),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  throw new Error(`Unable to locate sentinelguard.proto. Checked: ${candidates.join(', ')}`);
}

function createGrpcClient() {
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

function callUnary(method, request = {}) {
  return new Promise((resolve, reject) => {
    if (!grpcClient || typeof grpcClient[method] !== 'function') {
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

function stopAlertsStream() {
  if (alertsStream) {
    alertsStream.cancel();
    alertsStream.removeAllListeners();
    alertsStream = null;
  }
}

function startAlertsStream(sinceTimestamp = 0) {
  stopAlertsStream();

  if (!grpcClient) {
    throw new Error('gRPC client is not initialized');
  }

  alertsStream = grpcClient.getAlerts({ sinceTimestamp });

  alertsStream.on('data', (alert) => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send(ALERT_STREAM_CHANNEL, alert);
    }
  });

  alertsStream.on('error', (error) => {
    if (error.code === grpc.status.CANCELLED) {
      return;
    }
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send(ALERT_STREAM_CHANNEL, {
        __streamError: true,
        message: error.message,
      });
    }
  });

  alertsStream.on('end', () => {
    alertsStream = null;
  });
}

function registerIpcHandlers() {
  ipcMain.handle('sg:get-system-health', async () => callUnary('getSystemHealth', {}));
  ipcMain.handle('sg:get-process-risk-overview', async (_event, limit = 100) =>
    callUnary('getProcessRiskOverview', { limit })
  );
  ipcMain.handle('sg:get-quarantined-processes', async () =>
    callUnary('getQuarantinedProcesses', {})
  );
  ipcMain.handle('sg:get-detector-logs', async (_event, sinceTimestamp = 0, limit = ALERT_HISTORY_LIMIT) =>
    callUnary('getDetectorLogs', { sinceTimestamp, limit })
  );
  ipcMain.handle('sg:release-from-quarantine', async (_event, processId) =>
    callUnary('releaseFromQuarantine', { processId })
  );
  ipcMain.handle('sg:alerts:start', async (_event, sinceTimestamp = 0) => {
    startAlertsStream(sinceTimestamp);
    return { ok: true };
  });
  ipcMain.handle('sg:alerts:stop', async () => {
    stopAlertsStream();
    return { ok: true };
  });
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js'),
    },
  });

  const devUrl = process.env.ELECTRON_RENDERER_URL || 'http://localhost:5173';
  if (process.env.NODE_ENV === 'development' || process.env.ELECTRON_RENDERER_URL) {
    mainWindow.loadURL(devUrl);
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '../dist/index.html'));
  }

  mainWindow.on('closed', () => {
    stopAlertsStream();
    mainWindow = null;
  });
}

app.whenReady().then(() => {
  try {
    grpcClient = createGrpcClient();
  } catch (error) {
    console.error('Failed to initialize gRPC client:', error);
    grpcClient = null;
  }
  registerIpcHandlers();
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('before-quit', () => {
  stopAlertsStream();
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

