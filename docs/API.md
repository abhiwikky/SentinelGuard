# SentinelGuard API Documentation

## gRPC API (Agent to UI)

### Service: SentinelGuardService

#### GetSystemHealth
Returns current system health status.

**Request**:
```protobuf
message GetSystemHealthRequest {}
```

**Response**:
```protobuf
message SystemHealthResponse {
  bool agent_online = 1;
  bool driver_loaded = 2;
  int64 total_events = 3;
  int32 active_detections = 4;
  int32 quarantined_count = 5;
}
```

#### GetAlerts
Streams real-time alerts.

**Request**:
```protobuf
message GetAlertsRequest {
  int32 limit = 1;
  int64 since_timestamp = 2;
}
```

**Response**:
```protobuf
message Alert {
  string id = 1;
  int64 timestamp = 2;
  int32 process_id = 3;
  string process_path = 4;
  float ml_score = 5;
  bool quarantined = 6;
}
```

#### GetQuarantinedProcesses
Returns list of quarantined processes.

**Request**:
```protobuf
message GetQuarantinedProcessesRequest {}
```

**Response**:
```protobuf
message QuarantinedProcess {
  int32 process_id = 1;
  string process_path = 2;
  int64 quarantined_at = 3;
  string reason = 4;
}
```

## ALPC Communication (Kernel to Agent)

### Message Format

```c
typedef struct _FILE_EVENT {
    EVENT_TYPE Type;
    ULONG ProcessId;
    WCHAR ProcessPath[512];
    WCHAR FilePath[1024];
    ULONGLONG BytesRead;
    ULONGLONG BytesWritten;
    ULONGLONG Timestamp;
    ULONG Result;
    UCHAR EntropyPreview[16];
} FILE_EVENT;
```

### Event Types

- `EventFileCreate`
- `EventFileRead`
- `EventFileWrite`
- `EventFileRename`
- `EventFileDelete`
- `EventDirectoryEnum`
- `EventVSSDelete`
- `EventProcessCreate`
- `EventRegistryChange`

## Database Schema

### Events Table
```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    process_id INTEGER NOT NULL,
    process_path TEXT,
    file_path TEXT,
    bytes_read INTEGER,
    bytes_written INTEGER,
    timestamp INTEGER NOT NULL
);
```

### Alerts Table
```sql
CREATE TABLE alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    process_id INTEGER NOT NULL,
    ml_score REAL NOT NULL,
    quarantined INTEGER NOT NULL,
    timestamp INTEGER NOT NULL
);
```

## Configuration API

Configuration is stored in TOML format at:
```
C:\Program Files\SentinelGuard\config\config.toml
```

### Configuration Structure

```toml
[database]
path = "C:\\ProgramData\\SentinelGuard\\sentinelguard.db"

[ml]
model_path = "models\\ransomware_model.onnx"

[quarantine]
threshold = 0.7

[detectors]
entropy_threshold = 0.8
mass_write_threshold = 50
mass_write_window_seconds = 10
```

