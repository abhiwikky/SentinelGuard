import type { HealthStatus } from '../types';

interface Props {
  health: HealthStatus | null;
}

export default function HealthPanel({ health }: Props) {
  if (!health) {
    return (
      <div className="card mt-5 animate-fade-in">
        <div className="card-header">System Health</div>
        <div className="text-gray-500 text-sm">Loading health data...</div>
      </div>
    );
  }

  const stats = [
    {
      label: 'Events Processed',
      value: formatNumber(health.eventsProcessed),
      color: 'text-sentinel-400',
    },
    {
      label: 'Alerts Generated',
      value: formatNumber(health.alertsGenerated),
      color: parseInt(health.alertsGenerated) > 0 ? 'text-red-400' : 'text-green-400',
    },
    {
      label: 'Events/sec',
      value: health.eventsPerSecond,
      color: 'text-gray-300',
    },
    {
      label: 'Uptime',
      value: formatUptime(parseInt(health.uptimeSeconds)),
      color: 'text-gray-300',
    },
  ];

  const services = [
    { label: 'Agent', ok: health.agentRunning },
    { label: 'Driver', ok: health.driverConnected },
    { label: 'ML Model', ok: health.modelLoaded },
    { label: 'Database', ok: health.databaseConnected },
  ];

  return (
    <div className="card mt-5 animate-fade-in">
      <div className="flex items-center justify-between mb-4">
        <div className="card-header mb-0">System Health</div>
        <span className="text-xs text-gray-600 font-mono">v{health.agentVersion}</span>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-5">
        {stats.map((stat) => (
          <div key={stat.label} className="text-center">
            <div className={`stat-value ${stat.color}`}>{stat.value}</div>
            <div className="stat-label">{stat.label}</div>
          </div>
        ))}
      </div>

      {/* Service Status */}
      <div className="flex flex-wrap gap-3">
        {services.map((svc) => (
          <div
            key={svc.label}
            className={`flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium
              ${svc.ok
                ? 'bg-green-900/30 text-green-400 border border-green-800'
                : 'bg-red-900/30 text-red-400 border border-red-800'
              }`}
          >
            <div className={`w-1.5 h-1.5 rounded-full ${svc.ok ? 'bg-green-400' : 'bg-red-400'}`} />
            {svc.label}
          </div>
        ))}
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
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return `${h}h ${m}m`;
}
