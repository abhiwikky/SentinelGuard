export interface HealthStatus {
  agentRunning: boolean;
  driverConnected: boolean;
  modelLoaded: boolean;
  databaseConnected: boolean;
  eventsProcessed: string;
  alertsGenerated: string;
  uptimeSeconds: string;
  eventsPerSecond: string;
  agentVersion: string;
}

export interface Alert {
  alertId: string;
  processId: number;
  processName: string;
  severity: string;
  riskScore: number;
  description: string;
  detectorResults: DetectorResult[];
  quarantineStatus: string;
  timestampNs: string;
}

export interface DetectorResult {
  detectorName: string;
  score: number;
  evidence: string[];
  timestampNs: string;
  processId: number;
}

export interface ProcessRiskEntry {
  processId: number;
  processName: string;
  currentRiskScore: number;
  eventCount: string;
  lastEventNs: string;
  isQuarantined: boolean;
  detectorResults: DetectorResult[];
  weightedScore: number;
  mlScore: number;
}

export interface QuarantinedProcess {
  processId: number;
  processName: string;
  riskScore: number;
  quarantinedAtNs: string;
  status: string;
}

export interface BridgeHealth {
  bridge_running: boolean;
  grpc_connected: boolean;
  grpc_target: string;
  uptime_seconds: number;
}

export type ConnectionState = 'connected' | 'degraded' | 'disconnected';

// ─── Configuration Types ─────────────────────────────────────────────

export interface SentinelGuardConfig {
  agent: {
    version: string;
    log_level: string;
    event_buffer_size: number;
    health_report_interval_secs: number;
    process_whitelist: string[];
  };
  driver: {
    port_name: string;
    max_connections: number;
    max_message_size: number;
  };
  grpc: {
    listen_addr: string;
  };
  database: {
    path: string;
    wal_mode: boolean;
    max_size_mb: number;
  };
  detectors: {
    window_seconds: number;
    weights: {
      entropy_spike: number;
      mass_write: number;
      mass_rename_delete: number;
      ransom_note: number;
      shadow_copy: number;
      process_behavior: number;
      extension_explosion: number;
    };
    entropy: {
      threshold: number;
      min_file_size: number;
    };
    mass_write: {
      count_threshold: number;
      window_seconds: number;
    };
    mass_rename_delete: {
      count_threshold: number;
      window_seconds: number;
    };
    ransom_note: {
      patterns: string[];
    };
    shadow_copy: {
      suspicious_processes: string[];
    };
    process_behavior: {
      max_extensions: number;
      max_directories: number;
    };
    extension_explosion: {
      new_extension_threshold: number;
      window_seconds: number;
    };
  };
  quarantine: {
    helper_path: string;
    auto_quarantine_threshold: number;
    timeout_seconds: number;
  };
  inference: {
    model_path: string;
    num_features: number;
    fallback_enabled: boolean;
  };
  telemetry: {
    log_file: string;
    max_log_size_mb: number;
    max_log_files: number;
  };
}

export interface LogsResponse {
  lines: string[];
  total: number;
  path: string;
  error?: string;
}
