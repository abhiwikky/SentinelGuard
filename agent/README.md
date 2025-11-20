# SentinelGuard Rust Agent

## Overview

User-mode agent that receives kernel events, runs detectors, performs ML correlation, and triggers quarantine actions.

## Building

```bash
cargo build --release
```

## Running

```bash
# As service (Windows)
sc create SentinelGuardAgent binPath= "C:\Program Files\SentinelGuard\sentinelguard-agent.exe" start= auto
sc start SentinelGuardAgent

# As console app (for testing)
cargo run --release
```

## Architecture

- **Event Ingestion**: Receives events from kernel driver via ALPC
- **Detector Manager**: Runs multiple detectors in parallel
- **Correlation Engine**: Aggregates detector outputs
- **ML Inference**: ONNX-based final classification
- **Quarantine Controller**: Triggers C++ quarantine module
- **Telemetry Logger**: Writes to SQLite database

## Configuration

Edit `config/config.toml` for detector thresholds, ML model path, and other settings.

