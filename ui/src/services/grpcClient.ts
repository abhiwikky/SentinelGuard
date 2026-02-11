export interface Alert {
  id: number;
  processId: number;
  processPath: string;
  mlScore: number;
  quarantined: boolean;
  timestamp: number;
  triggeredDetectors: string[];
}

export interface ProcessRisk {
  processId: number;
  processPath: string;
  riskScore: number;
  lastActivity: number;
  activeDetectors: string[];
}

export interface ProcessRiskOverview {
  processes: ProcessRisk[];
}

export interface QuarantinedProcess {
  processId: number;
  processPath: string;
  mlScore: number;
  quarantinedAt: number;
  reason: string;
}

export interface QuarantinedProcessesResponse {
  processes: QuarantinedProcess[];
}

export interface DetectorLogEntry {
  detectorName: string;
  processId: number;
  score: number;
  timestamp: number;
  details: string;
}

export interface DetectorLogsResponse {
  entries: DetectorLogEntry[];
}

export interface SystemHealth {
  agentRunning: boolean;
  driverLoaded: boolean;
  eventsPerSecond: number;
  totalEvents: number;
  activeProcesses: number;
  quarantinedCount: number;
  cpuUsage: number;
  memoryUsage: number;
}

export interface ReleaseFromQuarantineResponse {
  success: boolean;
  message: string;
}

type Unsubscribe = () => void;

function requireApi() {
  if (!window.sentinelguardApi) {
    throw new Error('Electron preload API is unavailable');
  }
  return window.sentinelguardApi;
}

export const grpcClient = {
  async getSystemHealth(): Promise<SystemHealth> {
    return requireApi().getSystemHealth();
  },
  async getProcessRiskOverview(limit = 100): Promise<ProcessRiskOverview> {
    return requireApi().getProcessRiskOverview(limit);
  },
  async getQuarantinedProcesses(): Promise<QuarantinedProcessesResponse> {
    return requireApi().getQuarantinedProcesses();
  },
  async getDetectorLogs(sinceTimestamp = 0, limit = 100): Promise<DetectorLogsResponse> {
    return requireApi().getDetectorLogs(sinceTimestamp, limit);
  },
  async releaseFromQuarantine(processId: number): Promise<ReleaseFromQuarantineResponse> {
    return requireApi().releaseFromQuarantine(processId);
  },
  async startAlertStream(sinceTimestamp = 0): Promise<void> {
    await requireApi().startAlertStream(sinceTimestamp);
  },
  async stopAlertStream(): Promise<void> {
    await requireApi().stopAlertStream();
  },
  onAlert(callback: (payload: Alert | { __streamError: true; message: string }) => void): Unsubscribe {
    return requireApi().onAlert(callback);
  },
};
