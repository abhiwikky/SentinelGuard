import React, { createContext, useContext, useEffect, useState } from 'react';
import {
  grpcClient,
  type Alert,
  type ComponentHealth,
  type DashboardSnapshot,
  type DetectorLogsResponse,
  type ProcessRiskOverview,
  type QuarantinedProcessesResponse,
  type SystemHealth,
} from '../services/grpcClient';

interface DashboardDataContextValue {
  alerts: Alert[];
  streamError: string | null;
  snapshot: DashboardSnapshot | null;
  refreshError: string | null;
  refreshSnapshot: () => Promise<void>;
}

const DashboardDataContext = createContext<DashboardDataContextValue | null>(null);

const emptySnapshot: DashboardSnapshot = {
  fetchedAt: 0,
  componentHealth: {
    uiMode: 'browser',
    webBridgeRunning: false,
    grpcReachable: false,
    grpcAddress: '127.0.0.1:50051',
    agentRunning: false,
    driverLoaded: false,
    lastError: null,
  },
  systemHealth: null,
  processRiskOverview: { processes: [] },
  quarantinedProcesses: { processes: [] },
  detectorLogs: { entries: [] },
  errors: {
    systemHealth: null,
    processRiskOverview: null,
    quarantinedProcesses: null,
    detectorLogs: null,
  },
};

export function DashboardDataProvider({ children }: { children: React.ReactNode }) {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [streamError, setStreamError] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    const refresh = async () => {
      if (typeof document !== 'undefined' && document.visibilityState === 'hidden') {
        return;
      }

      try {
        const nextSnapshot = await grpcClient.getDashboardSnapshot();
        if (!mounted) {
          return;
        }
        setSnapshot(nextSnapshot);
        setRefreshError(null);
      } catch (error) {
        if (!mounted) {
          return;
        }
        setRefreshError(error instanceof Error ? error.message : 'Unable to refresh dashboard');
      }
    };

    const unsubscribe = grpcClient.onAlert((payload) => {
      if (!mounted) {
        return;
      }
      if ('__streamError' in payload) {
        setStreamError(payload.message);
        return;
      }
      setStreamError(null);
      setAlerts((current) => [payload, ...current].slice(0, 50));
    });

    void refresh();
    void grpcClient.startAlertStream(0).catch((error) => {
      if (mounted) {
        setStreamError(error instanceof Error ? error.message : 'Unable to start alert stream');
      }
    });

    const interval = setInterval(() => {
      void refresh();
    }, 5000);

    return () => {
      mounted = false;
      clearInterval(interval);
      unsubscribe();
      void grpcClient.stopAlertStream();
    };
  }, []);

  const value = {
    alerts,
    streamError,
    snapshot,
    refreshError,
    refreshSnapshot: async () => {
      const nextSnapshot = await grpcClient.getDashboardSnapshot();
      setSnapshot(nextSnapshot);
      setRefreshError(null);
    },
  };

  return (
    <DashboardDataContext.Provider value={value}>
      {children}
    </DashboardDataContext.Provider>
  );
}

export function useDashboardData() {
  const context = useContext(DashboardDataContext);
  if (!context) {
    throw new Error('useDashboardData must be used within a DashboardDataProvider');
  }
  return context;
}

export function useDashboardSnapshot(): DashboardSnapshot {
  const { snapshot } = useDashboardData();
  return snapshot ?? emptySnapshot;
}

export function useSystemHealthData(): SystemHealth | null {
  return useDashboardSnapshot().systemHealth;
}

export function useComponentHealthData(): ComponentHealth {
  return useDashboardSnapshot().componentHealth;
}

export function useProcessRiskData(): ProcessRiskOverview {
  return useDashboardSnapshot().processRiskOverview;
}

export function useQuarantinedProcessesData(): QuarantinedProcessesResponse {
  return useDashboardSnapshot().quarantinedProcesses;
}

export function useDetectorLogsData(): DetectorLogsResponse {
  return useDashboardSnapshot().detectorLogs;
}
