# SentinelGuard

**Windows-first ransomware detection and intervention platform for Windows 10/11 x64.**

SentinelGuard combines a kernel minifilter driver for real-time file-system telemetry, a Rust-based user-mode agent for behavioral analysis and ML-driven risk scoring, and a browser-based dashboard for operational visibility. When ransomware-like behavior is detected, the system automatically quarantines offending processes and persists forensic evidence.

## Architecture

```
┌──────────────┐     Filter Manager     ┌───────────────────┐
│   Minifilter │ ◄──── Comm Port ─────► │   Rust Agent      │
│   Driver     │                        │  ┌─────────────┐  │
│  (kernel)    │                        │  │ Detectors(7) │  │
└──────────────┘                        │  │ Correlation  │  │
                                        │  │ ONNX Infer.  │  │
                                        │  │ Quarantine   │  │
                                        │  │ SQLite       │  │
                                        │  │ gRPC Server  │  │
                                        │  └─────────────┘  │
                                        └────────┬──────────┘
                                                 │ gRPC :50051
                                        ┌────────▼──────────┐
                                        │  Node.js Bridge   │
                                        │  (HTTP/JSON/SSE)  │
                                        │  :3001             │
                                        └────────┬──────────┘
                                                 │ HTTP
                                        ┌────────▼──────────┐
                                        │  React Dashboard  │
                                        │  (Browser)        │
                                        └───────────────────┘
```

## Components

| Component | Language | Location |
|-----------|----------|----------|
| Kernel Minifilter Driver | C | `driver/` |
| User-Mode Agent | Rust | `agent/` |
| Quarantine Helper | C++ | `quarantine_helper/` |
| ML Training Pipeline | Python | `ml/` |
| gRPC Protobuf Defs | Protobuf | `proto/` |
| Node.js Web Bridge | JavaScript | `bridge/` |
| React Dashboard | TypeScript | `ui/` |
| Installer Scripts | PowerShell | `scripts/` |
| Configuration | TOML | `config/` |
| Tests | Python/Rust | `tests/` |

## Build Order

1. `cd agent && cargo build --release`
2. `cd driver && cmake -B build && cmake --build build` (requires WDK)
3. `cd quarantine_helper && cmake -B build && cmake --build build`
4. `cd ml && pip install -r requirements.txt && python train.py`
5. `cd ui && npm install && npm run build`
6. `cd bridge && npm install`
7. `powershell -ExecutionPolicy Bypass -File scripts\install.ps1`

## Requirements

- Windows 10/11 x64
- Rust 1.75+ with MSVC toolchain
- Visual Studio 2022 + WDK (for driver)
- CMake 3.20+
- Node.js 18+
- Python 3.10+
- ONNX Runtime 1.16+

## Configuration

Default configuration at `%ProgramData%\SentinelGuard\config.toml`. See `config/sentinelguard.toml` for the template.

## Security

- All network services bind to `127.0.0.1` only
- Driver communicates via Filter Manager communication ports (kernel-only IPC)
- Quarantine helper requires administrator privileges
- Agent runs as a Windows service under LOCAL SYSTEM
- No external network access required for core operation

## License

Proprietary. All rights reserved.
