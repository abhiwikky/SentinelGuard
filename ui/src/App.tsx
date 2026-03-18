import { useState, useEffect, useCallback } from 'react';
import type {
  HealthStatus,
  Alert,
  ProcessRiskEntry,
  QuarantinedProcess,
  DetectorResult,
  ConnectionState,
} from './types';
import { api, createAlertStream } from './api/client';
import Layout from './components/Layout';
import ConnectionStatus from './components/ConnectionStatus';
import HealthPanel from './components/HealthPanel';
import AlertFeed from './components/AlertFeed';
import ProcessRisk from './components/ProcessRisk';
import QuarantinePanel from './components/QuarantinePanel';
import DetectorLogs from './components/DetectorLogs';

const POLL_INTERVAL = 5000;

export default function App() {
  const [connection, setConnection] = useState<ConnectionState>('disconnected');
  const [health, setHealth] = useState<HealthStatus | null>(null);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [processes, setProcesses] = useState<ProcessRiskEntry[]>([]);
  const [quarantined, setQuarantined] = useState<QuarantinedProcess[]>([]);
  const [detectorLogs, setDetectorLogs] = useState<DetectorResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  const fetchAll = useCallback(async () => {
    try {
      const [bridgeHealth, healthData, alertData, processData, quarantineData, detectorData] =
        await Promise.allSettled([
          api.getBridgeHealth(),
          api.getHealth(),
          api.getAlerts(50),
          api.getProcesses(50),
          api.getQuarantined(),
          api.getDetectorLogs(50),
        ]);

      // Determine connection state
      if (bridgeHealth.status === 'rejected') {
        setConnection('disconnected');
        setError('Bridge is not reachable');
        return;
      }

      const bridge = bridgeHealth.value;
      if (!bridge.grpc_connected) {
        setConnection('degraded');
        setError('Bridge connected but gRPC backend unavailable');
      } else {
        setConnection('connected');
        setError(null);
      }

      if (healthData.status === 'fulfilled') setHealth(healthData.value);
      if (alertData.status === 'fulfilled') setAlerts(alertData.value);
      if (processData.status === 'fulfilled') setProcesses(processData.value);
      if (quarantineData.status === 'fulfilled') setQuarantined(quarantineData.value);
      if (detectorData.status === 'fulfilled') setDetectorLogs(detectorData.value);
    } catch (err) {
      setConnection('disconnected');
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  }, []);

  // Initial fetch and polling
  useEffect(() => {
    fetchAll();
    const interval = setInterval(fetchAll, POLL_INTERVAL);
    return () => clearInterval(interval);
  }, [fetchAll]);

  // SSE alert streaming
  useEffect(() => {
    const stream = createAlertStream(
      (alert) => {
        setAlerts((prev) => [alert, ...prev].slice(0, 100));
      },
      (errMsg) => {
        console.warn('SSE error:', errMsg);
      },
      () => {
        console.log('SSE connected');
      }
    );

    return () => stream.close();
  }, []);

  const handleRelease = async (processId: number) => {
    try {
      const result = await api.releaseProcess(processId);
      if (result.success) {
        fetchAll(); // Refresh data
      }
    } catch (err) {
      console.error('Release failed:', err);
    }
  };

  return (
    <Layout>
      {/* Connection Status Bar */}
      <ConnectionStatus state={connection} error={error} />

      {/* Health Overview */}
      <HealthPanel health={health} />

      {/* Main Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5 mt-5">
        {/* Alert Feed */}
        <div className="lg:col-span-2">
          <AlertFeed alerts={alerts} />
        </div>

        {/* Process Risk */}
        <ProcessRisk processes={processes} />

        {/* Quarantine */}
        <QuarantinePanel
          processes={quarantined}
          onRelease={handleRelease}
        />

        {/* Detector Logs */}
        <div className="lg:col-span-2">
          <DetectorLogs results={detectorLogs} />
        </div>
      </div>
    </Layout>
  );
}
