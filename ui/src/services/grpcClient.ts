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

export interface ComponentHealth {
  uiMode: 'browser';
  webBridgeRunning: boolean;
  grpcReachable: boolean;
  grpcAddress: string;
  agentRunning: boolean;
  driverLoaded: boolean;
  lastError: string | null;
}

export interface DashboardSnapshot {
  fetchedAt: number;
  componentHealth: ComponentHealth;
  systemHealth: SystemHealth | null;
  processRiskOverview: ProcessRiskOverview;
  quarantinedProcesses: QuarantinedProcessesResponse;
  detectorLogs: DetectorLogsResponse;
  errors: {
    systemHealth: string | null;
    processRiskOverview: string | null;
    quarantinedProcesses: string | null;
    detectorLogs: string | null;
  };
}

type AlertStreamError = { __streamError: true; message: string };
type AlertListener = (payload: Alert | AlertStreamError) => void;
type Unsubscribe = () => void;

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers || {}),
    },
  });

  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    try {
      const payload = await response.json();
      if (payload?.error) {
        message = String(payload.error);
      }
    } catch {
      // Leave the default message when the response is not JSON.
    }
    throw new Error(message);
  }

  return response.json() as Promise<T>;
}

const browserAlertListeners = new Set<AlertListener>();
let browserAlertSource: EventSource | null = null;

function closeBrowserAlertStream() {
  if (browserAlertSource) {
    browserAlertSource.close();
    browserAlertSource = null;
  }
}

function emitBrowserAlert(payload: Alert | AlertStreamError) {
  for (const listener of browserAlertListeners) {
    listener(payload);
  }
}

export const grpcClient = {
  async getSystemHealth(): Promise<SystemHealth> {
    return fetchJson<SystemHealth>('/api/system-health');
  },
  async getProcessRiskOverview(limit = 100): Promise<ProcessRiskOverview> {
    return fetchJson<ProcessRiskOverview>(`/api/process-risk-overview?limit=${limit}`);
  },
  async getQuarantinedProcesses(): Promise<QuarantinedProcessesResponse> {
    return fetchJson<QuarantinedProcessesResponse>('/api/quarantined-processes');
  },
  async getDetectorLogs(sinceTimestamp = 0, limit = 100): Promise<DetectorLogsResponse> {
    return fetchJson<DetectorLogsResponse>(
      `/api/detector-logs?sinceTimestamp=${sinceTimestamp}&limit=${limit}`
    );
  },
  async releaseFromQuarantine(processId: number): Promise<ReleaseFromQuarantineResponse> {
    return fetchJson<ReleaseFromQuarantineResponse>('/api/release-from-quarantine', {
      method: 'POST',
      body: JSON.stringify({ processId }),
    });
  },
  async getComponentHealth(): Promise<ComponentHealth> {
    return fetchJson<ComponentHealth>('/api/component-health');
  },
  async getDashboardSnapshot(): Promise<DashboardSnapshot> {
    return fetchJson<DashboardSnapshot>('/api/dashboard');
  },
  async startAlertStream(sinceTimestamp = 0): Promise<void> {
    closeBrowserAlertStream();
    browserAlertSource = new EventSource(`/api/alerts/stream?sinceTimestamp=${sinceTimestamp}`);

    browserAlertSource.onmessage = (event) => {
      emitBrowserAlert(JSON.parse(event.data) as Alert);
    };

    browserAlertSource.addEventListener('error', (event) => {
      const message =
        event instanceof MessageEvent && typeof event.data === 'string'
          ? event.data
          : 'Alert stream disconnected';
      emitBrowserAlert({ __streamError: true, message });
    });
  },
  async stopAlertStream(): Promise<void> {
    closeBrowserAlertStream();
  },
  onAlert(callback: AlertListener): Unsubscribe {
    browserAlertListeners.add(callback);
    return () => {
      browserAlertListeners.delete(callback);
    };
  },
};
