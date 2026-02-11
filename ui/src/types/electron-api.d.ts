import type {
  Alert,
  DetectorLogsResponse,
  ProcessRiskOverview,
  QuarantinedProcessesResponse,
  ReleaseFromQuarantineResponse,
  SystemHealth,
} from '../services/grpcClient';

declare global {
  interface Window {
    sentinelguardApi?: {
      getSystemHealth: () => Promise<SystemHealth>;
      getProcessRiskOverview: (limit?: number) => Promise<ProcessRiskOverview>;
      getQuarantinedProcesses: () => Promise<QuarantinedProcessesResponse>;
      getDetectorLogs: (sinceTimestamp?: number, limit?: number) => Promise<DetectorLogsResponse>;
      releaseFromQuarantine: (processId: number) => Promise<ReleaseFromQuarantineResponse>;
      startAlertStream: (sinceTimestamp?: number) => Promise<void>;
      stopAlertStream: () => Promise<void>;
      onAlert: (
        callback: (payload: Alert | { __streamError: true; message: string }) => void
      ) => () => void;
    };
  }
}

export {};
