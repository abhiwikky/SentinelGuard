# SentinelGuard Deployment Guide

## Prerequisites

- Windows 10/11 (64-bit)
- Administrator privileges
- Windows Driver Kit (WDK) 10 (for driver build)
- Visual Studio 2019 or later
- Rust toolchain (stable)
- Node.js 18+ (for UI build)

## Installation Steps

### 1. Build Components

#### Kernel Driver
```powershell
cd kernel
mkdir build
cd build
cmake .. -G "Visual Studio 16 2019" -A x64
cmake --build . --config Release
```

#### Rust Agent
```powershell
cd agent
cargo build --release
```

#### Quarantine Module
```powershell
cd quarantine
mkdir build
cd build
cmake .. -G "Visual Studio 16 2019" -A x64
cmake --build . --config Release
```

#### UI Dashboard
```powershell
cd ui
npm install
npm run build
```

### 2. Code Signing

**Important**: Kernel drivers must be signed before installation.

1. Obtain a code signing certificate
2. Sign the driver:
```powershell
signtool sign /f certificate.pfx /p password /t http://timestamp.digicert.com SentinelGuard.sys
```

3. Sign the agent and quarantine executables:
```powershell
signtool sign /f certificate.pfx /p password sentinelguard-agent.exe
signtool sign /f certificate.pfx /p password quarantine.exe
```

### 3. Install Kernel Driver

```powershell
# Copy driver to system directory
copy SentinelGuard.sys "C:\Windows\System32\drivers\"

# Install driver
sc create SentinelGuard type= kernel binPath= "C:\Windows\System32\drivers\SentinelGuard.sys"
sc start SentinelGuard

# Verify installation
sc query SentinelGuard
```

### 4. Install Rust Agent Service

```powershell
# Copy agent executable
copy sentinelguard-agent.exe "C:\Program Files\SentinelGuard\"

# Create service
sc create SentinelGuardAgent binPath= "C:\Program Files\SentinelGuard\sentinelguard-agent.exe" start= auto
sc start SentinelGuardAgent

# Verify service
sc query SentinelGuardAgent
```

### 5. Install Quarantine Module

```powershell
copy quarantine.exe "C:\Program Files\SentinelGuard\"
```

### 6. Install UI Dashboard

```powershell
cd ui
npm run build
# Install Electron app using electron-builder or manual installation
```

### 7. Configuration

Edit `C:\Program Files\SentinelGuard\config\config.toml`:

```toml
[database]
path = "C:\\ProgramData\\SentinelGuard\\sentinelguard.db"

[ml]
model_path = "C:\\Program Files\\SentinelGuard\\models\\ransomware_model.onnx"

[quarantine]
threshold = 0.7

[detectors]
entropy_threshold = 0.8
mass_write_threshold = 50
```

### 8. ML Model

Place the trained ONNX model at:
```
C:\Program Files\SentinelGuard\models\ransomware_model.onnx
```

## Verification

1. Check driver status:
```powershell
sc query SentinelGuard
```

2. Check agent service:
```powershell
sc query SentinelGuardAgent
```

3. Verify database creation:
```powershell
dir "C:\ProgramData\SentinelGuard\"
```

4. Launch UI and verify connection

## Uninstallation

```powershell
# Stop services
sc stop SentinelGuardAgent
sc stop SentinelGuard

# Delete services
sc delete SentinelGuardAgent
sc delete SentinelGuard

# Remove files
rmdir /s "C:\Program Files\SentinelGuard"
rmdir /s "C:\ProgramData\SentinelGuard"
```

## Troubleshooting

### Driver Not Loading
- Verify code signing
- Check Event Viewer for errors
- Ensure Windows Test Signing is enabled (for test builds)

### Agent Not Starting
- Check service logs
- Verify configuration file exists
- Ensure database directory is writable

### No Events Detected
- Verify driver is loaded: `sc query SentinelGuard`
- Check ALPC connection in agent logs
- Verify file operations are being intercepted

## Maintenance

- Regular database cleanup (configure retention period)
- ML model updates
- Detector rule updates
- Log rotation

