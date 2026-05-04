/**
 * SentinelGuard Node.js Web Bridge
 *
 * Translates gRPC calls from the Rust agent into browser-safe
 * HTTP/JSON endpoints and SSE streams.
 *
 * Endpoints:
 *   GET  /api/health           - System health status
 *   GET  /api/alerts           - Recent alerts (?limit=N&since_ns=N)
 *   GET  /api/alerts/stream    - SSE stream of real-time alerts
 *   GET  /api/processes        - Process risk overview
 *   GET  /api/quarantined      - Quarantined processes
 *   POST /api/quarantined/release - Release a process (?process_id=N)
 *   GET  /api/detectors        - Detector logs (?limit=N&since_ns=N)
 *   GET  /api/bridge/health    - Bridge connectivity status
 *
 * All endpoints return JSON. The bridge listens on
 * 127.0.0.1:3001 by default (localhost only).
 */

const express = require("express");
const cors = require("cors");
const path = require("path");
const fs = require("fs");
const TOML = require("@iarna/toml");
const { createClient, callUnary, DEFAULT_TARGET } = require("./grpc_client");

const PORT = parseInt(process.env.BRIDGE_PORT || "3001", 10);
const HOST = process.env.BRIDGE_HOST || "127.0.0.1";
const GRPC_TARGET = process.env.GRPC_TARGET || DEFAULT_TARGET;
const CONFIG_PATH = process.env.CONFIG_PATH || path.resolve(__dirname, "..", "config", "sentinelguard.toml");

const app = express();
app.use(cors({ origin: true }));
app.use(express.json());

// Create gRPC client
let grpcClient = null;
let grpcConnected = false;

function ensureClient() {
  if (!grpcClient) {
    grpcClient = createClient(GRPC_TARGET);
  }
  return grpcClient;
}

// Test gRPC connectivity
async function checkGrpcHealth() {
  try {
    const client = ensureClient();
    await callUnary(client, "getHealth", {});
    grpcConnected = true;
    return true;
  } catch (err) {
    grpcConnected = false;
    return false;
  }
}

// Periodic health check
setInterval(checkGrpcHealth, 5000);

// ─── API Routes ──────────────────────────────────────────────────────

// Bridge health
app.get("/api/bridge/health", (req, res) => {
  res.json({
    bridge_running: true,
    grpc_connected: grpcConnected,
    grpc_target: GRPC_TARGET,
    uptime_seconds: Math.floor(process.uptime()),
  });
});

// System health
app.get("/api/health", async (req, res) => {
  try {
    const client = ensureClient();
    const response = await callUnary(client, "getHealth", {});
    grpcConnected = true;
    res.json(response.health || {});
  } catch (err) {
    grpcConnected = false;
    res.status(503).json({
      error: "Backend unavailable",
      details: err.message,
    });
  }
});

// Alerts
app.get("/api/alerts", async (req, res) => {
  try {
    const client = ensureClient();
    const response = await callUnary(client, "getAlerts", {
      limit: parseInt(req.query.limit || "50", 10),
      sinceNs: req.query.since_ns || "0",
    });
    grpcConnected = true;
    res.json(response.alerts || []);
  } catch (err) {
    grpcConnected = false;
    res.status(503).json({ error: "Backend unavailable", details: err.message });
  }
});

// Alert SSE stream
app.get("/api/alerts/stream", (req, res) => {
  res.writeHead(200, {
    "Content-Type": "text/event-stream",
    "Cache-Control": "no-cache",
    Connection: "keep-alive",
    "X-Accel-Buffering": "no",
  });

  res.write("data: {\"type\":\"connected\"}\n\n");

  const client = ensureClient();
  let stream = null;

  try {
    stream = client.streamAlerts({ sinceNs: req.query.since_ns || "0" });

    stream.on("data", (alert) => {
      res.write(`data: ${JSON.stringify(alert)}\n\n`);
    });

    stream.on("error", (err) => {
      if (err.code !== 1) {
        // Not CANCELLED
        res.write(
          `data: ${JSON.stringify({ type: "error", message: err.message })}\n\n`
        );
      }
    });

    stream.on("end", () => {
      res.write('data: {"type":"stream_ended"}\n\n');
      res.end();
    });
  } catch (err) {
    res.write(
      `data: ${JSON.stringify({ type: "error", message: err.message })}\n\n`
    );
  }

  req.on("close", () => {
    if (stream) {
      stream.cancel();
    }
  });
});

// Process risk overview
app.get("/api/processes", async (req, res) => {
  try {
    const client = ensureClient();
    const response = await callUnary(client, "getProcessRisk", {
      limit: parseInt(req.query.limit || "50", 10),
    });
    grpcConnected = true;
    res.json(response.processes || []);
  } catch (err) {
    grpcConnected = false;
    res.status(503).json({ error: "Backend unavailable", details: err.message });
  }
});

// Quarantined processes
app.get("/api/quarantined", async (req, res) => {
  try {
    const client = ensureClient();
    const response = await callUnary(client, "getQuarantined", {});
    grpcConnected = true;
    res.json(response.processes || []);
  } catch (err) {
    grpcConnected = false;
    res.status(503).json({ error: "Backend unavailable", details: err.message });
  }
});

// Release quarantined process
app.post("/api/quarantined/release", async (req, res) => {
  const processId = parseInt(req.body.process_id || req.query.process_id || "0", 10);

  if (!processId) {
    return res.status(400).json({ error: "process_id is required" });
  }

  try {
    const client = ensureClient();
    const response = await callUnary(client, "releaseProcess", {
      processId: processId,
    });
    grpcConnected = true;
    res.json(response);
  } catch (err) {
    grpcConnected = false;
    res.status(503).json({ error: "Backend unavailable", details: err.message });
  }
});

// Detector logs
app.get("/api/detectors", async (req, res) => {
  try {
    const client = ensureClient();
    const response = await callUnary(client, "getDetectorLogs", {
      limit: parseInt(req.query.limit || "50", 10),
      sinceNs: req.query.since_ns || "0",
    });
    grpcConnected = true;
    res.json(response.results || []);
  } catch (err) {
    grpcConnected = false;
    res.status(503).json({ error: "Backend unavailable", details: err.message });
  }
});

// ─── Configuration Management ────────────────────────────────────────

// Get current configuration
app.get("/api/config", (req, res) => {
  try {
    if (!fs.existsSync(CONFIG_PATH)) {
      return res.status(404).json({ error: "Config file not found", path: CONFIG_PATH });
    }
    const raw = fs.readFileSync(CONFIG_PATH, "utf-8");
    const config = TOML.parse(raw);
    res.json(config);
  } catch (err) {
    res.status(500).json({ error: "Failed to read config", details: err.message });
  }
});

// Update configuration
app.put("/api/config", (req, res) => {
  try {
    const newConfig = req.body;
    if (!newConfig || typeof newConfig !== "object") {
      return res.status(400).json({ error: "Invalid config payload" });
    }

    // Read existing config to preserve comments structure
    if (!fs.existsSync(CONFIG_PATH)) {
      return res.status(404).json({ error: "Config file not found", path: CONFIG_PATH });
    }

    // Validate critical fields
    if (newConfig.agent && newConfig.agent.log_level) {
      const validLevels = ["trace", "debug", "info", "warn", "error"];
      if (!validLevels.includes(newConfig.agent.log_level)) {
        return res.status(400).json({ error: `Invalid log_level. Must be one of: ${validLevels.join(", ")}` });
      }
    }

    if (newConfig.quarantine && newConfig.quarantine.auto_quarantine_threshold !== undefined) {
      const t = newConfig.quarantine.auto_quarantine_threshold;
      if (typeof t !== "number" || t < 0 || t > 1) {
        return res.status(400).json({ error: "auto_quarantine_threshold must be a number between 0 and 1" });
      }
    }

    // Backup existing config
    const backupPath = CONFIG_PATH + ".bak";
    fs.copyFileSync(CONFIG_PATH, backupPath);

    // Write new config
    const tomlString = TOML.stringify(newConfig);
    fs.writeFileSync(CONFIG_PATH, tomlString, "utf-8");

    res.json({ success: true, message: "Configuration saved. Agent restart required for changes to take effect." });
  } catch (err) {
    res.status(500).json({ error: "Failed to write config", details: err.message });
  }
});

// Get agent logs
app.get("/api/logs", (req, res) => {
  try {
    // Try to read log path from config
    let logPath = "C:\\ProgramData\\SentinelGuard\\logs\\sentinelguard.log";
    try {
      if (fs.existsSync(CONFIG_PATH)) {
        const config = TOML.parse(fs.readFileSync(CONFIG_PATH, "utf-8"));
        if (config.telemetry && config.telemetry.log_file) {
          logPath = config.telemetry.log_file;
        }
      }
    } catch (_) { /* use default */ }

    if (!fs.existsSync(logPath)) {
      return res.json({ lines: [], error: `Log file not found: ${logPath}` });
    }

    const maxLines = parseInt(req.query.lines || "200", 10);
    const content = fs.readFileSync(logPath, "utf-8");
    const allLines = content.split("\n");
    const lines = allLines.slice(-maxLines);

    res.json({ lines, total: allLines.length, path: logPath });
  } catch (err) {
    res.status(500).json({ error: "Failed to read logs", details: err.message });
  }
});

// ─── Static File Serving ─────────────────────────────────────────────

// Serve the built UI in production
// install.ps1 copies dist/* into ../ui/ (no dist subfolder), so check both locations
const uiDistPath = path.resolve(__dirname, "..", "ui", "dist");
const uiFlatPath = path.resolve(__dirname, "..", "ui");
const uiBuildPath = fs.existsSync(path.join(uiDistPath, "index.html")) ? uiDistPath : uiFlatPath;
app.use(express.static(uiBuildPath));

// Fallback to index.html for SPA routing
app.get("*", (req, res) => {
  if (!req.path.startsWith("/api/")) {
    res.sendFile(path.join(uiBuildPath, "index.html"), (err) => {
      if (err) {
        res.status(404).json({ error: "UI build not found. Run 'npm run build' in ui/ directory." });
      }
    });
  }
});

// ─── Start Server ────────────────────────────────────────────────────

app.listen(PORT, HOST, () => {
  console.log(`SentinelGuard Web Bridge running on http://${HOST}:${PORT}`);
  console.log(`gRPC target: ${GRPC_TARGET}`);
  console.log(`UI build: ${uiBuildPath}`);

  // Initial health check
  checkGrpcHealth().then((connected) => {
    console.log(`gRPC backend: ${connected ? "connected" : "not available"}`);
  });
});
