# SentinelGuard Threat Model

## Threat Categories

### 1. Ransomware Attacks

**Threat**: Malicious software that encrypts user files and demands payment.

**Detection Methods**:
- Entropy spike detection
- Mass file write operations
- File extension changes
- Ransom note creation
- Shadow copy deletion

**Mitigation**:
- Real-time process suspension
- File isolation
- Alert generation

### 2. System Tampering

**Threat**: Attempts to disable or bypass SentinelGuard.

**Attack Vectors**:
- Driver unloading
- Service termination
- Process injection
- Configuration modification

**Mitigation**:
- Code signing
- Tamper detection
- Process protection
- Secure configuration storage

### 3. False Positives

**Threat**: Legitimate applications triggering false alarms.

**Mitigation**:
- Whitelist management
- ML-based correlation
- Configurable thresholds
- User review process

## Security Assumptions

1. Kernel driver runs with SYSTEM privileges
2. User agent runs as Windows service
3. Communication channels are secure (ALPC, gRPC with mTLS)
4. Configuration files are protected
5. Code signing prevents tampering

## Attack Surface

### Kernel Driver
- File operation interception
- ALPC communication
- Driver loading/unloading

### User Agent
- Event processing
- Detector execution
- Quarantine actions
- Database access

### UI Dashboard
- gRPC communication
- Local file access
- Configuration changes

## Defense in Depth

1. **Kernel-level monitoring**: First line of defense
2. **Multi-detector analysis**: Redundant detection methods
3. **ML correlation**: Advanced pattern recognition
4. **Automated response**: Rapid quarantine
5. **Audit logging**: Forensic analysis

## Limitations

- Cannot detect fileless ransomware (memory-only)
- May have false positives with encryption software
- Requires administrator privileges
- Performance impact on high I/O systems

