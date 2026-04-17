import type { PageId } from '../App';
import type { ConnectionState } from '../types';
import { Wifi, WifiOff, AlertTriangle } from 'lucide-react';

interface TopBarProps {
  activePage: PageId;
  connection: ConnectionState;
  error: string | null;
}

const PAGE_TITLES: Record<PageId, string> = {
  dashboard: 'Dashboard Overview',
  processes: 'Process Risk Analysis',
  alerts: 'Alert Feed',
  quarantine: 'Quarantine Management',
};

export default function TopBar({ activePage, connection, error }: TopBarProps) {
  const connectionConfig = {
    connected: {
      icon: Wifi,
      label: 'All systems operational',
      color: 'var(--accent-green)',
      dotClass: 'ok',
    },
    degraded: {
      icon: AlertTriangle,
      label: 'Degraded connectivity',
      color: 'var(--accent-orange)',
      dotClass: 'warn',
    },
    disconnected: {
      icon: WifiOff,
      label: 'Disconnected',
      color: 'var(--accent-red)',
      dotClass: 'error',
    },
  }[connection];

  const Icon = connectionConfig.icon;

  return (
    <header
      className="shrink-0 flex items-center justify-between px-6 py-4"
      style={{
        background: 'var(--bg-surface)',
        borderBottom: '1px solid rgba(255,255,255,0.04)',
      }}
    >
      {/* Page Title */}
      <div>
        <h1 className="text-lg font-bold tracking-tight" style={{ color: 'var(--text-primary)', margin: 0 }}>
          {PAGE_TITLES[activePage]}
        </h1>
        <p className="text-xs mt-0.5" style={{ color: 'var(--text-muted)', margin: 0 }}>
          {new Date().toLocaleDateString(undefined, {
            weekday: 'long',
            year: 'numeric',
            month: 'short',
            day: 'numeric',
          })}
        </p>
      </div>

      {/* Connection Status */}
      <div className="flex items-center gap-3">
        {error && (
          <span
            className="text-xs font-medium max-w-[200px] truncate"
            style={{ color: 'var(--text-muted)' }}
            title={error}
          >
            {error}
          </span>
        )}
        <div
          className="neu-flat-sm flex items-center gap-2 px-3 py-2 cursor-default"
          title={connectionConfig.label}
        >
          <div className={`status-dot ${connectionConfig.dotClass}`} />
          <Icon size={14} style={{ color: connectionConfig.color }} />
          <span className="text-xs font-semibold" style={{ color: connectionConfig.color }}>
            {connectionConfig.label}
          </span>
        </div>
      </div>
    </header>
  );
}
