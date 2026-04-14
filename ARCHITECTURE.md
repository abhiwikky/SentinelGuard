# SentinelGuard Architecture Document

## 1. Project Overview

SentinelGuard is a Windows-first ransomware detection and intervention system for Windows 10/11 x64. It combines a kernel minifilter driver for real-time file-system telemetry, a Rust-based user-mode agent with 7 behavioral detectors and ONNX ML inference, a C++ quarantine helper for process suspension, and a browser-based React dashboard connected through a Node.js web bridge. When ransomware-like behavior exceeds a configurable risk threshold, the system automatically  quarantines the offending process and persists forensic evidence.

## 2. Goals and Non-Goals

### Goals
- Real-time interception of file-system operations via kernel minifilter
- Behavioral detection through 7 independent, scoring detectors
- ML-enhanced risk scoring via ONNX Random Forest model
- Automatic process quarantine when threshold exceeded
- Local-only gRPC API for operational tooling
- Browser UI for system visibility with near-real-time updates via SSE
- Production-ready logging, configuration, and deployment

### Non-Goals
- Network-based threat detection (out of scope)
- Cloud telemetry upload (all processing is local)
- Anti-virus signature scanning
- User-mode API hooking
- Cross-platform support
- Real-time file content scanning (entropy is computed when available, not on every I/O)

## 3. High-Level Architecture

```
 ┌──────────────────────────────────────────────────────────────┐
 │                       KERNEL MODE                           │
 │  ┌─────────────────────────────┐                            │
 │  │   SentinelGuard Minifilter  │                            │
 │  │   (sentinelguard.sys)       │                            │
 │  │                             │                            │
 │  │  Pre-Op Callbacks:          │                            │
 │  │   Create, Write, SetInfo,   │                            │
 │  │   DirectoryControl          │                            │
 │  │                             │                            │
 │  │  → SG_EVENT struct          │                            │
 │  │  → FltSendMessage           │                            │
 │  └─────────┬───────────────────┘                            │
 │            │ Filter Manager Communication Port              │
 └────────────┼────────────────────────────────────────────────┘
              │
 ┌────────────┼────────────────────────────────────────────────┐
 │            ▼           USER MODE                            │
 │  ┌─────────────────────────────────────────┐                │
 │  │        Rust Agent                       │                │
 │  │        (sentinelguard_agent.exe)        │                │
 │  │                                         │                │
 │  │  ┌──────────┐  ┌──────────────┐         │                │
 │  │  │ Comms    │→ │ Event Queue  │         │                │
 │  │  │ Module   │  │ (mpsc chan)  │         │                │
 │  │  └──────────┘  └──────┬───────┘         │                │
 │  │                       ▼                 │                │
 │  │  ┌──────────────────────────┐           │                │
 │  │  │ Detector Framework (7x)  │           │                │
 │  │  │  entropy | mass_write    │           │                │
 │  │  │  rename  | ransom_note   │           │                │
 │  │  │  shadow  | behavior      │           │                │
 │  │  │  extension_explosion     │           │                │
 │  │  └──────────┬───────────────┘           │                │
 │  │             ▼                           │                │
 │  │  ┌──────────────────┐                   │                │
 │  │  │ Correlator       │─→ Weighted Agg    │                │
 │  │  └────────┬─────────┘                   │                │
 │  │           ▼                             │                │
 │  │  ┌────────────────┐                     │                │
 │  │  │ ONNX Inference │─→ ML Score          │                │
 │  │  └────────┬───────┘                     │                │
 │  │           ▼                             │                │
 │  │  final_score > threshold?               │                │
 │  │     YES → Quarantine Helper             │                │
 │  │     → Alert → DB → gRPC broadcast       │                │
 │  │                                         │                │
 │  │  ┌──────────┐  ┌──────────┐             │                │
 │  │  │ SQLite   │  │ gRPC     │             │                │
 │  │  │ Database │  │ :50051   │             │                │
 │  │  └──────────┘  └────┬─────┘             │                │
 │  └──────────────────────┼──────────────────┘                │
 │                         │                                   │
 │  ┌──────────────────────┼──────────────────┐                │
 │  │  Node.js Bridge      ▼                  │                │
 │  │  (server.js)   HTTP/JSON/SSE :3001      │                │
 │  └──────────────────────┬──────────────────┘                │
 │                         │                                   │
 │  ┌──────────────────────┼──────────────────┐                │
 │  │  React Dashboard     ▼                  │                │
 │  │  (Browser)    http://127.0.0.1:3001     │                │
 │  └─────────────────────────────────────────┘                │
 └─────────────────────────────────────────────────────────────┘
```

## 4. Component Specifications

### 4.1 Kernel Minifilter Driver

- **Purpose**: Intercept file system operations and forward telemetry to user mode
- **Language**: C, targeting WDK with Filter Manager
- **Files**: `driver/src/driver.c`, `driver/src/communication.c`, `driver/src/operations.c`
- **Inputs**: File system IRPs (Create, Write, SetInformation, DirectoryControl)
- **Outputs**: `SG_EVENT` structs via `FltSendMessage` to communication port
- **Key design**: Altitude 370050 (Activity Monitor class), admin-only security descriptor on port, 100ms send timeout, silent drop if no client connected, NTFS/ReFS only attachment
- **Error handling**: All NTSTATUS codes checked, KdPrintEx for debug logging, graceful unload
- **Performance**: Pre-operation callbacks only (no post-op), kernel-mode callers skipped, paging I/O skipped

### 4.2 Rust Agent

- **Purpose**: Central processing engine for events, detection, correlation, and response
- **Modules**: communication, config, events, detectors (7), correlation, inference, quarantine, database, grpc_server, telemetry, security
- **Key design**: Tokio async runtime, mpsc channel for backpressure-aware ingestion, watch channel for shutdown coordination, broadcast channel for alert SSE streaming
- **Startup**: Config load → Telemetry init → DB open → Detectors init → Correlator → Inference engine → Quarantine manager → Driver connection → gRPC server → Event processing loop
- **Shutdown**: Ctrl+C → watch channel signal → graceful task completion → 2s drain → exit

### 4.3 Detector Framework

7 detectors, each implementing the `Detector` trait returning `DetectorResult` with score ∈ [0.0, 1.0]:

| Detector | Trigger | Scoring Strategy |
|----------|---------|-----------------|
| entropy_spike | Write ops with entropy > threshold | Base score from entropy magnitude + repetition factor |
| mass_write | Write count in time window | Ramp from threshold/2 to 2x threshold |
| mass_rename_delete | Rename/delete count in window | Tiered scoring at 50% and 100% of threshold |
| ransom_note | File creation matching note patterns | 0.7 for first match, +0.1 per additional |
| shadow_copy | ShadowCopyDelete op or suspicious process | 0.8+ for direct detection, context-dependent for process names |
| process_behavior | Unique extensions and directories | Weighted combination: 60% extensions, 40% directories |
| extension_explosion | Unique new extensions created | Ramp above threshold |

### 4.4 ML Pipeline

- **Training**: Python with scikit-learn Random Forest on synthetic data (beta distributions)
- **Features**: 7 detector scores (same order as `FEATURE_NAMES` in `features.py`)
- **Export**: ONNX via skl2onnx with opset 13
- **Inference**: `ort` crate in Rust, fallback to weighted average if model unavailable
- **Final scoring**: `final_score = weighted_score * 0.4 + ml_score * 0.6`

### 4.5 Quarantine Helper

- **Purpose**: Suspend/resume processes via NtSuspendProcess/NtResumeProcess from ntdll.dll
- **CLI**: `--suspend <PID>` or `--release <PID>`
- **Exit codes**: 0=success, 1=invalid args, 2=process not found, 3=access denied, 4=already in state, 5=internal error

### 4.6 gRPC API (Protobuf)

- **Services**: GetHealth, GetAlerts, StreamAlerts (server-streaming), GetProcessRisk, GetQuarantined, ReleaseProcess, GetDetectorLogs
- **Binding**: 127.0.0.1:50051 (loopback only)

### 4.7 Node.js Web Bridge

- **Purpose**: Translate gRPC to browser-safe HTTP/JSON and SSE
- **Endpoints**: /api/health, /api/alerts, /api/alerts/stream (SSE), /api/processes, /api/quarantined, /api/quarantined/release, /api/detectors, /api/bridge/health
- **Binding**: 127.0.0.1:3001, serves built UI static files

### 4.8 React Dashboard

- **Components**: Layout, ConnectionStatus, HealthPanel, AlertFeed, ProcessRisk, QuarantinePanel, DetectorLogs
- **Data flow**: Polling every 5s + SSE for real-time alerts
- **States**: Connected, Degraded (bridge up, gRPC down), Disconnected

## 5. Data Contracts

### Kernel Event (C struct → Rust)
```
SG_EVENT { StructSize, Operation, ProcessId, Timestamp, FileSize, FilePath[520], NewFilePath[520], ProcessName[260], FileExtension[32] }
```

### SQLite Schema
- **events**: event_id, process_id, process_name, operation, file_path, new_file_path, file_size, entropy, timestamp_ns, file_extension
- **alerts**: alert_id, process_id, process_name, severity, risk_score, description, quarantine_status, timestamp_ns
- **detector_results**: id, alert_id, detector_name, score, evidence(JSON), timestamp_ns, process_id
- **quarantine_log**: id, process_id, process_name, risk_score, action, status, timestamp_ns

## 6. Build and Packaging Workflow

| Step | Command | Input | Output |
|------|---------|-------|--------|
| 1 | `cd agent && cargo build --release` | Rust source + proto | `target/release/sentinelguard_agent.exe` |
| 2 | `cd driver` + VS2022/WDK build | C source | `sentinelguard.sys` |
| 3 | `cd quarantine_helper && cmake -B build && cmake --build build --config Release` | C++ source | `quarantine_helper.exe` |
| 4 | `cd ml && pip install -r requirements.txt && python train.py --output model.onnx` | Python + deps | `model.onnx` |
| 5 | `cd ui && npm install && npm run build` | React/TS source | `ui/dist/` |
| 6 | `cd bridge && npm install` | package.json | `node_modules/` |
| 7 | `powershell -ExecutionPolicy Bypass -File scripts\install.ps1` | All artifacts | Installed system |

> **IMPORTANT: Deployment Note** 
> The `install.ps1` and `build.ps1` scripts do **not** automatically sync development changes to the ML model or the configuration file over to the system runtime directory.
> 
> *   **ML Model:** If you retrain the model (`cd ml && python train.py`), you must manually copy `ml\model.onnx` to `C:\ProgramData\SentinelGuard\model.onnx`.
> *   **Configuration:** If you modify `config\sentinelguard.toml`, you must manually copy it to `C:\ProgramData\SentinelGuard\config.toml`.
> 
> Restart the agent (`Stop-Process -Name sentinelguard_agent -Force` or restart the service) for changes to take effect.

## 7. Security Model

- **Trust boundary**: Kernel ↔ User mode via Filter Manager (admin-only port)
- **Network**: All services bound to 127.0.0.1 only
- **Quarantine**: Requires PROCESS_SUSPEND_RESUME privilege (admin)
- **Signing**: Driver requires kernel-mode code signing; agent and helper should be Authenticode signed
- **Config**: Stored in ProgramData with system ACLs
- **Defense**: No external network calls, no cloud dependencies

## 8. Verification Notes

- Protobuf field names use consistent camelCase in JSON (proto-loader default)
- SQLite schema column names match Rust insert/query parameterization
- ONNX input name "input" matches training export; output accessed by "output" or "probabilities"
- Bridge endpoints match UI API client paths exactly
- Config TOML keys match Rust serde field names
- Driver SG_EVENT struct layout matches Rust RawSgEvent repr(C)
- Detector names in weights config match detector `name()` return values
- Full E2E flow requires WDK-equipped machine with test signing for kernel component
