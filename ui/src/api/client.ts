/**
 * SentinelGuard API Client
 *
 * Communicates with the Node.js bridge via HTTP/JSON and SSE.
 */

import type {
  HealthStatus,
  Alert,
  DetectorResult,
  ProcessRiskEntry,
  QuarantinedProcess,
  BridgeHealth,
} from '../types';

const API_BASE = '/api';

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${url}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`API error ${response.status}: ${body}`);
  }

  return response.json();
}

export const api = {
  getHealth: () => fetchJson<HealthStatus>('/health'),

  getBridgeHealth: () => fetchJson<BridgeHealth>('/bridge/health'),

  getAlerts: (limit = 50, sinceNs = '0') =>
    fetchJson<Alert[]>(`/alerts?limit=${limit}&since_ns=${sinceNs}`),

  getProcesses: (limit = 50) =>
    fetchJson<ProcessRiskEntry[]>(`/processes?limit=${limit}`),

  getQuarantined: () => fetchJson<QuarantinedProcess[]>('/quarantined'),

  releaseProcess: (processId: number) =>
    fetchJson<{ success: boolean; message: string }>('/quarantined/release', {
      method: 'POST',
      body: JSON.stringify({ process_id: processId }),
    }),

  getDetectorLogs: (limit = 50, sinceNs = '0') =>
    fetchJson<DetectorResult[]>(`/detectors?limit=${limit}&since_ns=${sinceNs}`),
};

/**
 * Create an SSE connection for real-time alert streaming
 */
export function createAlertStream(
  onAlert: (alert: Alert) => void,
  onError: (error: string) => void,
  onConnected: () => void
): EventSource {
  const eventSource = new EventSource(`${API_BASE}/alerts/stream`);

  eventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      if (data.type === 'connected') {
        onConnected();
      } else if (data.type === 'error') {
        onError(data.message);
      } else if (data.processId !== undefined) {
        onAlert(data as Alert);
      }
    } catch {
      // Ignore parse errors from heartbeats
    }
  };

  eventSource.onerror = () => {
    onError('SSE connection lost');
  };

  return eventSource;
}
