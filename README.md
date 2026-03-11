# SentinelGuard

Real-time ransomware detection and intervention for Windows. The repository contains a kernel minifilter driver, a Rust user-mode agent, an ML training pipeline, a quarantine helper, and a browser-first React dashboard with a local gRPC web bridge.

## Repository Layout

- `kernel/`: Windows minifilter driver built with CMake and the WDK
- `agent/`: Rust agent with detector pipeline, SQLite logging, and gRPC server
- `quarantine/`: native helper executable used to suspend or release processes
- `ml/`: Python training pipeline that exports an ONNX model
- `ui/`: browser UI and local Node.js gRPC bridge
- `scripts/`: install, uninstall, and signing scripts
- `tests/`: end-to-end simulator scripts

## Current State

The old README had drifted from the codebase. These points reflect the repository as it exists now:

- The top-level docs had encoding corruption; this file is now ASCII-only.
- The kernel and quarantine components are C/C++ projects built with CMake. The kernel build prefers a WDK CMake package and falls back to `WDK_ROOT`.
- The ML training script currently writes `ml/models/sentinelguard_model.onnx`.
- The UI is browser-only and runs as a separate process from the agent.
- The installer copies the browser UI bundle, local bridge, proto file, and UI dependencies so the dashboard is launchable after install.
- `agent/src/config.rs` currently returns built-in defaults; `agent/config/config.toml` is installed, but the agent does not parse it yet.
- The dashboard uses one shared snapshot refresh loop plus alert streaming, which reduces redundant polling across tabs.
- The dashboard can verify bridge and gRPC reachability in addition to agent-reported health data.
- Some gRPC-backed UI panels are still placeholders; see `PROJECT_STATUS.md` for gaps.

## Prerequisites

Build everything on Windows 10/11 x64.

- Visual Studio 2022 with Desktop development with C++
- CMake 3.15+
- Windows Driver Kit 10
- Rust stable toolchain
- Node.js 18+ and npm
- Python 3.10+
- Administrator privileges for driver installation and service creation

Optional environment variables:

- `WDK_ROOT`: override the default WDK install path if it is not under `C:\Program Files (x86)\Windows Kits\10`

## Full Build

Run the steps from the repository root `C:\Users\abhij\SentinelGuard`.

### 1. Build the Rust agent

```powershell
cd agent
cargo build --release
cd ..
```

Expected artifact:

- `agent\target\release\sentinelguard-agent.exe`

### 2. Build the kernel driver

```powershell
cd kernel
New-Item -ItemType Directory -Force build | Out-Null
cmake -S . -B build -G "Visual Studio 17 2022" -A x64
cmake --build build --config Release
cd ..
```

Expected artifact:

- `kernel\build\Release\SentinelGuard.sys`

Notes:

- If CMake cannot find the WDK automatically, pass `-DWDK_ROOT="C:\Program Files (x86)\Windows Kits\10"` or your installed WDK path.
- A production install still requires code signing for the driver.

### 3. Build the quarantine helper

```powershell
cd quarantine
New-Item -ItemType Directory -Force build | Out-Null
cmake -S . -B build -G "Visual Studio 17 2022" -A x64
cmake --build build --config Release
cd ..
```

Expected artifact:

- `quarantine\build\Release\quarantine.exe`

### 4. Train the ML model

```powershell
cd ml
python -m pip install --upgrade pip
pip install -r requirements.txt
python train_model.py
cd ..
```

Expected artifacts:

- `ml\models\sentinelguard_model.onnx`
- `ml\models\random_forest.joblib`
- `ml\models\scaler.joblib`

Important:

- The training script currently generates synthetic data. It is useful for wiring the pipeline, not for production accuracy claims.

### 5. Install UI dependencies

```powershell
cd ui
npm install
cd ..
```

### 6. Build the browser UI

Build the React bundle that the local web bridge serves:

```powershell
cd ui
npm run build:web
cd ..
```

Expected artifact:

- `ui\dist\`

### 7. Launch the browser UI locally

If you are running from the repository checkout, start the local Node.js bridge and open the dashboard in a browser:

```powershell
cd ui
npm run web
```

Then open:

```text
http://localhost:4173
```

What this does:

- serves the built React app from `ui\dist`
- proxies browser requests to the agent gRPC server at `127.0.0.1:50051` by default
- keeps the UI isolated from SentinelGuard itself, so a browser crash does not affect the agent process
- exposes component health checks so the UI can show bridge status, gRPC reachability, agent status, and driver status

## One-Pass Build Order

If you want the entire repository built in the order required by the current scripts:

1. `agent`: `cargo build --release`
2. `kernel`: `cmake -S . -B build ...` then `cmake --build build --config Release`
3. `quarantine`: `cmake -S . -B build ...` then `cmake --build build --config Release`
4. `ml`: `pip install -r requirements.txt` then `python train_model.py`
5. `ui`: `npm install` then `npm run build:web`

That sequence produces all artifacts consumed by `scripts\install.ps1`.

## Installation

After the builds finish, run the installer from an elevated PowerShell session:

```powershell
.\scripts\install.ps1
```

The installer currently expects these inputs:

- `agent\target\release\sentinelguard-agent.exe`
- `kernel\build\Release\SentinelGuard.sys`
- `quarantine\build\Release\quarantine.exe`
- `ui\dist\`
- `ui\server.js`
- `ui\start-web.ps1`
- `ui\package.json`
- `ui\node_modules\`
- `agent\proto\sentinelguard.proto`
- `ml\models\*.onnx`
- `agent\config\config.toml`

What it does:

- creates `C:\Program Files\SentinelGuard`
- copies the agent, driver, quarantine helper, browser UI bundle, browser bridge, UI dependencies, config, proto, and ONNX model
- creates the `SentinelGuard` kernel service unless `-SkipDriver` is used
- creates the `SentinelGuardAgent` service unless `-SkipAgentService` is used
- creates `C:\ProgramData\SentinelGuard`

After install, launch the UI with:

```powershell
powershell -ExecutionPolicy Bypass -File "C:\Program Files\SentinelGuard\ui\start-web.ps1"
```

Then open:

```text
http://localhost:4173
```

Use only one launch path at a time:

- repository checkout: `cd ui` then `npm run web`
- installed copy under `C:\Program Files\SentinelGuard`: run `start-web.ps1`

Current limitations:

- The driver must be signed before Windows will load it outside test scenarios.
- The agent executable is not implemented as a native Windows service yet; the installer already warns that service start may fail.
- The browser UI can verify bridge reachability and agent-reported health, but some data panels are still backed by placeholder gRPC responses.

## Signing

To sign the built binaries:

```powershell
.\scripts\sign_binaries.ps1 -CertificatePath "cert.pfx" -CertificatePassword "password"
```

The script signs:

- `agent\target\release\sentinelguard-agent.exe`
- `quarantine\build\Release\quarantine.exe`
- `kernel\build\Release\SentinelGuard.sys`

## Testing

Agent tests:

```powershell
cd agent
cargo test
cd ..
```

End-to-end simulator:

```powershell
python tests\e2e_test.py
```

Current limitation:

- The E2E script is a simulator and does not yet validate full agent-driver-dashboard behavior automatically.

## Useful References

- `PROJECT_STATUS.md`: implemented areas and known gaps
- `docs/ARCHITECTURE.md`: architecture notes
- `docs/API.md`: gRPC surface
- `docs/DEPLOYMENT.md`: deployment-oriented notes
- `kernel/README.md`, `agent/README.md`, `quarantine/README.md`, `ml/README.md`: component-specific docs

## License

This project is licensed under the MIT License. See `LICENSE`.
