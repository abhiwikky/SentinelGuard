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
