# SentinelGuard Deployment Guide

This guide matches the repository as of March 11, 2026. It focuses on building the current components and installing the artifacts expected by `scripts/install.ps1`.

## Prerequisites

- Windows 10/11 x64
- Visual Studio 2022 with Desktop development with C++
- CMake 3.15+
- Windows Driver Kit 10
- Rust stable
- Python 3.10+
- Node.js 18+
- Administrator privileges for installation

Optional:

- `WDK_ROOT` if your WDK is not installed under the default Windows Kits path

## Build Steps

### Rust agent

```powershell
cd agent
cargo build --release
cd ..
```

Artifact:

- `agent\target\release\sentinelguard-agent.exe`

### Kernel driver

```powershell
cd kernel
New-Item -ItemType Directory -Force build | Out-Null
cmake -S . -B build -G "Visual Studio 17 2022" -A x64
cmake --build build --config Release
cd ..
```

Artifact:

- `kernel\build\Release\SentinelGuard.sys`

### Quarantine helper

```powershell
cd quarantine
New-Item -ItemType Directory -Force build | Out-Null
cmake -S . -B build -G "Visual Studio 17 2022" -A x64
cmake --build build --config Release
cd ..
```

Artifact:

- `quarantine\build\Release\quarantine.exe`

### ML model

```powershell
cd ml
python -m pip install --upgrade pip
pip install -r requirements.txt
python train_model.py
cd ..
```

Artifacts:

- `ml\models\sentinelguard_model.onnx`
- `ml\models\random_forest.joblib`
- `ml\models\scaler.joblib`

### UI renderer

```powershell
cd ui
npm install
npm run build:react
cd ..
```

Artifact:

- `ui\dist\`

Optional Electron packaging:

```powershell
cd ui
npm run build
cd ..
```

Note:

- `scripts/install.ps1` expects `ui\dist`, not the packaged Electron output.

## Install

Run from an elevated PowerShell session:

```powershell
.\scripts\install.ps1
```

The installer copies:

- agent executable
- quarantine helper
- kernel driver
- renderer bundle from `ui\dist`
- ONNX model from `ml\models`
- `agent\config\config.toml`

It also creates:

- `C:\Program Files\SentinelGuard`
- `C:\ProgramData\SentinelGuard`
- `SentinelGuard` kernel service
- `SentinelGuardAgent` service unless `-SkipAgentService` is used

## Signing

Before loading the kernel driver on a normal Windows system, sign it:

```powershell
.\scripts\sign_binaries.ps1 -CertificatePath "cert.pfx" -CertificatePassword "password"
```

## Verification

Verify artifacts exist before install:

```powershell
Test-Path agent\target\release\sentinelguard-agent.exe
Test-Path kernel\build\Release\SentinelGuard.sys
Test-Path quarantine\build\Release\quarantine.exe
Test-Path ui\dist
Get-ChildItem ml\models\*.onnx
```

Verify services after install:

```powershell
sc.exe query SentinelGuard
sc.exe query SentinelGuardAgent
```

## Current Limitations

- The agent currently uses built-in defaults from `agent/src/config.rs`; installed TOML values are not parsed yet.
- The agent binary is not implemented as a full Windows service yet, so service startup can fail.
- The installed UI is only the renderer bundle, not a packaged Electron desktop application.
- Several gRPC and dashboard features are still placeholders.
