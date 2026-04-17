import { useState, useEffect, useCallback, createContext, useContext } from 'react';
import type {
  HealthStatus,
  Alert,
  ProcessRiskEntry,
  QuarantinedProcess,
  ConnectionState,
} from './types';
import { api, createAlertStream } from './api/client';
import Sidebar from './components/Sidebar';
import TopBar from './components/TopBar';
import DashboardPage from './pages/DashboardPage';
import ProcessRiskPage from './pages/ProcessRiskPage';
import AlertsPage from './pages/AlertsPage';
import QuarantinePage from './pages/QuarantinePage';

const POLL_INTERVAL = 5000;

export type PageId = 'dashboard' | 'processes' | 'alerts' | 'quarantine';

// ─── Global Data Context ───
interface AppData {
  connection: ConnectionState;
  health: HealthStatus | null;
  alerts: Alert[];
  processes: ProcessRiskEntry[];
  quarantined: QuarantinedProcess[];
  error: string | null;
  handleRelease: (pid: number) => Promise<void>;
  refreshAll: () => void;
}

export const AppDataContext = createContext<AppData>({
  connection: 'disconnected',
  health: null,
  alerts: [],
  processes: [],
  quarantined: [],
  error: null,
  handleRelease: async () => {},
  refreshAll: () => {},
});

export const useAppData = () => useContext(AppDataContext);

export default function App() {
  const [activePage, setActivePage] = useState<PageId>('dashboard');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [connection, setConnection] = useState<ConnectionState>('disconnected');
  const [health, setHealth] = useState<HealthStatus | null>(null);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [processes, setProcesses] = useState<ProcessRiskEntry[]>([]);
  const [quarantined, setQuarantined] = useState<QuarantinedProcess[]>([]);
  const [error, setError] = useState<string | null>(null);

  const fetchAll = useCallback(async () => {
    try {
      const [bridgeHealth, healthData, alertData, processData, quarantineData] =
        await Promise.allSettled([
          api.getBridgeHealth(),
          api.getHealth(),
          api.getAlerts(50),
          api.getProcesses(50),
          api.getQuarantined(),
        ]);

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
    } catch (err) {
      setConnection('disconnected');
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  }, []);

  useEffect(() => {
    fetchAll();
    const interval = setInterval(fetchAll, POLL_INTERVAL);
    return () => clearInterval(interval);
  }, [fetchAll]);

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
        fetchAll();
      }
    } catch (err) {
      console.error('Release failed:', err);
    }
  };

  const renderPage = () => {
    switch (activePage) {
      case 'dashboard':
        return <DashboardPage />;
      case 'processes':
        return <ProcessRiskPage />;
      case 'alerts':
        return <AlertsPage />;
      case 'quarantine':
        return <QuarantinePage />;
      default:
        return <DashboardPage />;
    }
  };

  const contextValue: AppData = {
    connection,
    health,
    alerts,
    processes,
    quarantined,
    error,
    handleRelease,
    refreshAll: fetchAll,
  };

  return (
    <AppDataContext.Provider value={contextValue}>
      <div className="flex h-screen overflow-hidden">
        <Sidebar
          activePage={activePage}
          onNavigate={setActivePage}
          collapsed={sidebarCollapsed}
          onToggleCollapse={() => setSidebarCollapsed(!sidebarCollapsed)}
          alertCount={alerts.length}
          quarantineCount={quarantined.length}
        />
        <div className="flex-1 flex flex-col overflow-hidden">
          <TopBar
            activePage={activePage}
            connection={connection}
            error={error}
          />
          <main className="flex-1 overflow-y-auto custom-scroll p-6" style={{ background: 'var(--bg-base)' }}>
            {renderPage()}
          </main>
        </div>
      </div>
    </AppDataContext.Provider>
  );
}
