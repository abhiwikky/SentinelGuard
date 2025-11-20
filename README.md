# SentinelGuard — Real-Time Ransomware Detection & Intervention System

[![CI/CD](https://github.com/yourusername/SentinelGuard/workflows/CI/badge.svg)](https://github.com/yourusername/SentinelGuard/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**SentinelGuard** is an enterprise-grade, real-time ransomware detection and intervention system for Windows. It combines kernel-level monitoring, multi-detector analysis, machine learning correlation, and automated quarantine to protect systems from ransomware attacks.

## 🎯 Features

- **Kernel-Level Monitoring**: Windows minifilter driver intercepts all file operations in real-time
- **Multi-Detector Analysis**: 7 specialized detectors analyze entropy, mass operations, ransom notes, and more
- **ML-Powered Correlation**: ONNX-based machine learning engine correlates detector outputs for accurate classification
- **Automated Quarantine**: Instant process suspension and file isolation upon detection
- **Real-Time Dashboard**: Electron-based UI with live alerts and system health monitoring
- **ETW Integration**: Extended monitoring via Event Tracing for Windows
- **Secure Communication**: gRPC with mTLS for agent-UI communication
- **Comprehensive Logging**: SQLite database for audit trails and telemetry

## 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│      Electron/React UI Dashboard       │
└────────────────┬────────────────────────┘
                 │ gRPC
                 ▼
┌─────────────────────────────────────────┐
│      Rust User-Mode Agent               │
│  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ Event    │→ │ Detector │→ │   ML   │ │
│  │ Ingestion│  │ Manager  │  │ Engine │ │
│  └──────────┘  └──────────┘  └────────┘ │
└────────┬─────────────────────────────────┘
         │ ALPC/Named Pipes
         ▼
┌─────────────────────────────────────────┐
│   Kernel Minifilter Driver (C++)        │
│   + ETW Providers                       │
└─────────────────────────────────────────┘
```

## 📦 Components

### 1. Kernel Minifilter Driver (`kernel/`)
- **Language**: C++
- **Technology**: Windows Filter Manager (FltMgr), ETW
- **Purpose**: Intercept file system operations, monitor process creation, detect VSS deletion attempts
- **Key Features**:
  - File create, read, write, rename, delete interception
  - Process path and file path extraction
  - Entropy calculation for write operations
  - ALPC communication with user-mode agent

### 2. Rust User-Mode Agent (`agent/`)
- **Language**: Rust
- **Technology**: Tokio async runtime, ONNX Runtime, gRPC (Tonic)
- **Purpose**: Event processing, detector execution, ML inference, quarantine triggering
- **Modules**:
  - Event Ingestion: Receives and batches kernel events
  - Detector Manager: Runs 7 detectors in parallel
  - Correlation Engine: ONNX-based ML inference
  - Quarantine Controller: Triggers C++ quarantine module
  - gRPC Server: Serves UI dashboard
  - Database Logger: SQLite telemetry storage

### 3. Detectors (`agent/src/detectors/`)
Each detector produces a risk score (0.0 - 1.0):

1. **Entropy Spike Detector**: Detects rapid increases in file entropy (encryption indicator)
2. **Mass Write Detector**: Identifies bulk file modifications within time windows
3. **Mass Rename/Delete Detector**: Flags rename/delete storms
4. **Ransom Note Detector**: Pattern matching for common ransom note text (YARA)
5. **Shadow Copy Deletion Detector**: Monitors VSS deletion attempts
6. **Process Behavior Detector**: Analyzes suspicious process behavior (DLL loads, API calls)
7. **File Extension Explosion Detector**: Detects known ransomware extensions (.locked, .encrypted, etc.)

### 4. ML Correlation Engine (`ml/`)
- **Training**: Python (scikit-learn, LightGBM)
- **Inference**: Rust (ONNX Runtime)
- **Features**: 15 features including detector scores and derived metrics
- **Model**: RandomForest classifier exported to ONNX format

### 5. Quarantine Module (`quarantine/`)
- **Language**: C++
- **Technology**: NT Native APIs
- **Actions**:
  - Suspend malicious processes (`NtSuspendProcess`)
  - Block file handles
  - Isolate written files
  - Set file ACLs to read-only

### 6. UI Dashboard (`ui/`)
- **Technology**: Electron + React + Tailwind CSS
- **Features**:
  - Live alerts feed
  - Process risk overview
  - Quarantined process management
  - Detector logs
  - System health monitoring
  - Configuration management

## 🚀 Quick Start

### Prerequisites

- **Windows 10/11** (x64)
- **Visual Studio 2019+** with C++ Desktop Development
- **Windows Driver Kit (WDK) 10**
- **Rust** (stable, latest)
- **Node.js** 18+ and npm
- **Python** 3.10+ (for ML training)
- **Administrator privileges** (for driver installation)

### Building

#### 1. Build Rust Agent

```powershell
cd agent
cargo build --release
```

#### 2. Build Kernel Driver

```powershell
cd kernel
mkdir build
cd build
cmake .. -G "Visual Studio 17 2022" -A x64
cmake --build . --config Release
```

#### 3. Build Quarantine Module

```powershell
cd quarantine
mkdir build
cd build
cmake .. -G "Visual Studio 17 2022" -A x64
cmake --build . --config Release
```

#### 4. Train ML Model

```powershell
cd ml
pip install -r requirements.txt
python train_model.py
```

#### 5. Build UI

```powershell
cd ui
npm install
npm run build
```

### Installation

Run the installation script with administrator privileges:

```powershell
.\scripts\install.ps1
```

This will:
- Install kernel driver service
- Install agent as Windows service
- Copy binaries and configuration
- Set up directories and permissions

**Note**: The kernel driver must be signed with a valid code signing certificate for production use.

### Code Signing

Sign all binaries before deployment:

```powershell
.\scripts\sign_binaries.ps1 -CertificatePath "cert.pfx" -CertificatePassword "password"
```

## 🔧 Configuration

Edit `agent/config/config.toml`:

```toml
[database]
path = "C:\\ProgramData\\SentinelGuard\\sentinelguard.db"

[ml]
model_path = "models\\sentinelguard_model.onnx"
quarantine_threshold = 0.7

[detectors]
entropy_threshold = 0.8
mass_write_threshold = 50
mass_write_window_seconds = 10
```

## 🧪 Testing

### Unit Tests

```powershell
cd agent
cargo test
```

### Integration Tests

```powershell
cd agent
cargo test --test integration_test
```

### End-to-End Tests

```powershell
python tests/e2e_test.py
```

## 📊 Usage

### Starting Services

```powershell
# Start kernel driver
sc start SentinelGuard

# Start agent (auto-starts as service)
sc start SentinelGuardAgent
```

### Viewing Logs

```powershell
# Agent logs
Get-Content "C:\Program Files\SentinelGuard\logs\agent.log" -Tail 50

# Database queries
sqlite3 "C:\ProgramData\SentinelGuard\sentinelguard.db" "SELECT * FROM alerts ORDER BY timestamp DESC LIMIT 10;"
```

### Launching UI

```powershell
cd "C:\Program Files\SentinelGuard\ui"
.\sentinelguard-ui.exe
```

## 🔒 Security Features

- **Code Signing**: All binaries signed with trusted certificates
- **Tamper Detection**: Periodic integrity checks on agent, driver, and config
- **Process Protection**: Anti-debug and process protection mechanisms
- **Secure Communication**: gRPC with mTLS (optional)
- **Access Control**: Restricted file permissions and service isolation

## 📚 Documentation

- [Architecture Documentation](docs/ARCHITECTURE.md)
- [API Reference](docs/API.md)
- [Deployment Guide](docs/DEPLOYMENT.md)
- [ML Features Reference](docs/ML_FEATURES.md)
- [Threat Model](docs/THREAT_MODEL.md)

## 🛠️ Development

### Project Structure

```
SentinelGuard/
├── agent/              # Rust user-mode agent
│   ├── src/
│   │   ├── detectors/  # Detector modules
│   │   ├── correlation.rs
│   │   ├── database.rs
│   │   └── ...
│   └── Cargo.toml
├── kernel/             # C++ minifilter driver
│   ├── SentinelGuard.c
│   ├── Events.c
│   └── ETW.c
├── quarantine/         # C++ quarantine module
├── ml/                 # Python ML training
│   └── train_model.py
├── ui/                 # Electron/React UI
├── scripts/            # Installation/signing scripts
├── tests/              # E2E tests
└── docs/               # Documentation
```

### CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`) builds:
- Rust agent
- Kernel driver
- Quarantine module
- ML model
- UI dashboard
- Creates installer package

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⚠️ Disclaimer

**This software is for educational and research purposes. Use in production environments requires:**
- Valid code signing certificates
- Thorough testing in your environment
- Compliance with local regulations
- Proper security audits

## 🙏 Acknowledgments

- Windows Filter Manager (FltMgr) documentation
- ONNX Runtime team
- Rust and Tokio communities
- All contributors

## 📧 Contact

For questions, issues, or contributions, please open an issue on GitHub.

---

**SentinelGuard** — Protecting systems from ransomware, one detection at a time.

