# SentinelGuard Architecture

## Overview

SentinelGuard is a multi-layered ransomware detection system that operates at both kernel and user mode to provide comprehensive protection.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Electron/React UI                        │
│              (Monitoring & Configuration)                   │
└───────────────────────┬─────────────────────────────────────┘
                        │ gRPC / WebSocket
                        v
┌─────────────────────────────────────────────────────────────┐
│              Rust User-Mode Agent                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Event      │  │  Detector    │  │  Correlation │     │
│  │  Ingestion   │→ │   Manager    │→ │    Engine    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│         │                  │                    │           │
│         └──────────────────┴────────────────────┘           │
│                            │                                │
│                            v                                │
│                   ┌─────────────────┐                      │
│                   │  Quarantine     │                      │
│                   │   Controller    │                      │
│                   └─────────────────┘                      │
└───────────────────────┬─────────────────────────────────────┘
                        │ ALPC / Named Pipes
                        v
┌─────────────────────────────────────────────────────────────┐
│            Kernel Minifilter Driver (C++)                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  File Operation Interception (FltMgr)               │   │
│  │  - Create, Read, Write, Rename, Delete              │   │
│  │  - VSS Deletion Detection                           │   │
│  │  - Process Monitoring                               │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Kernel Minifilter Driver

**Technology**: Windows Filter Manager (FltMgr), C++

**Responsibilities**:
- Intercept all file system operations
- Monitor shadow copy deletion attempts
- Track process creation and file access patterns
- Stream events to user-mode agent via ALPC

**Key Features**:
- Low-latency event capture
- Minimal performance impact
- Secure communication channel

### 2. Rust User-Mode Agent

**Technology**: Rust, Tokio async runtime

**Responsibilities**:
- Receive and batch kernel events
- Run multiple detectors in parallel
- Aggregate detector scores
- Perform ML-based correlation
- Trigger quarantine actions
- Log all events to SQLite

**Modules**:
- Event Ingestion
- Detector Manager
- Correlation Engine
- Quarantine Controller
- Database Logger

### 3. Detectors

Each detector analyzes events and produces a risk score (0.0 - 1.0):

1. **Entropy Spike Detector**: Detects rapid increases in file entropy
2. **Mass Write Detector**: Identifies bulk file modifications
3. **Mass Rename/Delete Detector**: Flags rename/delete storms
4. **Ransom Note Detector**: Pattern matching for ransom notes
5. **Shadow Copy Deletion Detector**: Monitors VSS deletion attempts
6. **Process Behavior Detector**: Analyzes suspicious process behavior
7. **File Extension Explosion Detector**: Detects known ransomware extensions

### 4. ML Correlation Engine

**Technology**: ONNX Runtime (inference), Python (training)

**Features**:
- Aggregates detector outputs
- Temporal pattern analysis
- Final ransomware probability score
- Threshold-based quarantine trigger

### 5. Quarantine Module

**Technology**: C++, NT Native APIs

**Actions**:
- Suspend malicious processes (NtSuspendProcess)
- Block file handles
- Isolate written files
- Set file ACLs to read-only

### 6. UI Dashboard

**Technology**: Electron + React + Tailwind CSS

**Features**:
- Real-time alert feed
- Process risk overview
- Quarantined process management
- Detector logs
- System health monitoring

## Data Flow

1. **Kernel Event** → File operation detected by minifilter
2. **Event Streaming** → Event sent to user-mode agent via ALPC
3. **Event Ingestion** → Agent receives and stores event
4. **Detector Analysis** → All detectors analyze event in parallel
5. **Score Aggregation** → Detector scores collected per process
6. **ML Correlation** → ONNX model produces final score
7. **Quarantine Decision** → If score > threshold, trigger quarantine
8. **Alert Generation** → Alert logged and sent to UI
9. **UI Update** → Dashboard displays real-time alert

## Security Considerations

- Code signing for kernel driver
- Secure communication channels (ALPC, gRPC with mTLS)
- Tamper detection mechanisms
- Process protection
- Encrypted configuration storage

## Performance

- Kernel driver: < 1% CPU overhead
- User agent: < 5% CPU overhead
- Event latency: < 10ms kernel to agent
- Quarantine response: < 100ms from detection

