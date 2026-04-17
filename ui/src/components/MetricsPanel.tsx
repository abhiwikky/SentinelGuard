import type { HealthStatus } from '../types';
import {
  AlertTriangle,
  Zap,
  Clock,
  Server,
  Cpu,
  Database,
  HardDrive,
} from 'lucide-react';

interface Props {
  health: HealthStatus | null;
}

export default function MetricsPanel({ health }: Props) {
  if (!health) {
    return (
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="metric-card">
            <div className="skeleton h-4 w-20 mb-3" />
            <div className="skeleton h-8 w-16" />
          </div>
        ))}
      </div>
    );
  }

  const metrics = [
    {
      label: 'Active Alerts',
      value: formatNumber(health.alertsGenerated),
      icon: AlertTriangle,
      color: parseInt(health.alertsGenerated) > 0 ? 'var(--accent-red)' : 'var(--accent-green)',
      glow: parseInt(health.alertsGenerated) > 0,
    },
    {
      label: 'Events / Sec',
      value: health.eventsPerSecond,
      icon: Zap,
      color: 'var(--accent-cyan)',
      glow: false,
    },
    {
      label: 'Uptime',
      value: formatUptime(parseInt(health.uptimeSeconds)),
      icon: Clock,
      color: 'var(--accent-blue)',
      glow: false,
    },
  ];

  const services = [
    { label: 'Agent', ok: health.agentRunning, icon: Server },
    { label: 'Minifilter', ok: health.driverConnected, icon: HardDrive },
    { label: 'ML Engine', ok: health.modelLoaded, icon: Cpu },
    { label: 'Database', ok: health.databaseConnected, icon: Database },
  ];

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 animate-fade-in">
      {/* Key Metrics */}
      {metrics.map((m) => {
        const Icon = m.icon;
        return (
          <div key={m.label} className="metric-card cursor-default">
            <div className="flex items-center justify-between mb-3">
              <span className="card-title">{m.label}</span>
              <div
                className="neu-pressed-sm flex items-center justify-center"
                style={{ width: 32, height: 32 }}
              >
                <Icon size={15} style={{ color: m.color }} />
              </div>
            </div>
            <div
              className="text-2xl font-bold font-mono"
              style={{
                color: m.color,
                textShadow: m.glow ? `0 0 12px ${m.color}40` : 'none',
              }}
            >
              {m.value}
            </div>
          </div>
        );
      })}

      {/* Service Health */}
      <div className="metric-card cursor-default">
        <div className="flex items-center justify-between mb-3">
          <span className="card-title">Services</span>
        </div>
        <div className="grid grid-cols-2 gap-2.5">
          {services.map((svc) => {
            const SvcIcon = svc.icon;
            return (
              <div key={svc.label} className="flex items-center gap-2">
                <div className={`status-dot ${svc.ok ? 'ok' : 'error'}`} />
                <SvcIcon size={12} style={{ color: 'var(--text-muted)', flexShrink: 0 }} />
                <span className="text-xs font-medium" style={{ color: svc.ok ? 'var(--text-secondary)' : 'var(--accent-red)' }}>
                  {svc.label}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function formatNumber(n: string): string {
  const num = parseInt(n) || 0;
  if (num >= 1_000_000) return `${(num / 1_000_000).toFixed(1)}M`;
  if (num >= 1_000) return `${(num / 1_000).toFixed(1)}K`;
  return num.toString();
}

function formatUptime(seconds: number): string {
  if (isNaN(seconds)) return '—';
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return `${h}h ${m}m`;
}
