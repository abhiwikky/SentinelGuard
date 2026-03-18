import type { Alert } from '../types';

interface Props {
  alerts: Alert[];
}

function severityBadge(severity: string) {
  const map: Record<string, string> = {
    SEVERITY_CRITICAL: 'badge-critical',
    SEVERITY_HIGH: 'badge-high',
    SEVERITY_MEDIUM: 'badge-medium',
    SEVERITY_LOW: 'badge-low',
    '4': 'badge-critical',
    '3': 'badge-high',
    '2': 'badge-medium',
    '1': 'badge-low',
  };
  return map[severity] || 'badge-low';
}

function severityLabel(severity: string) {
  const map: Record<string, string> = {
    SEVERITY_CRITICAL: 'CRITICAL',
    SEVERITY_HIGH: 'HIGH',
    SEVERITY_MEDIUM: 'MEDIUM',
    SEVERITY_LOW: 'LOW',
    '4': 'CRITICAL',
    '3': 'HIGH',
    '2': 'MEDIUM',
    '1': 'LOW',
  };
  return map[severity] || severity;
}

function formatTime(ns: string): string {
  const ms = parseInt(ns) / 1_000_000;
  if (isNaN(ms) || ms === 0) return '—';
  return new Date(ms).toLocaleTimeString();
}

export default function AlertFeed({ alerts }: Props) {
  return (
    <div className="card animate-fade-in">
      <div className="flex items-center justify-between mb-3">
        <div className="card-header mb-0">Alert Feed</div>
        {alerts.length > 0 && (
          <div className="flex items-center gap-1.5">
            <div className="live-dot bg-red-400" />
            <span className="text-xs text-red-400">Live</span>
          </div>
        )}
      </div>

      {alerts.length === 0 ? (
        <div className="text-center py-8 text-gray-600">
          <svg className="w-10 h-10 mx-auto mb-2 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <p className="text-sm">No alerts — system is clean</p>
        </div>
      ) : (
        <div className="space-y-2 max-h-96 overflow-y-auto">
          {alerts.map((alert, i) => (
            <div
              key={`${alert.alertId}-${i}`}
              className="flex items-start gap-3 p-3 rounded-lg bg-gray-800/50 hover:bg-gray-800/80 transition-colors"
            >
              <div className="mt-0.5">
                <span className={`badge ${severityBadge(alert.severity)}`}>
                  {severityLabel(alert.severity)}
                </span>
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-medium text-sm text-white truncate">
                    {alert.processName || 'Unknown'}
                  </span>
                  <span className="text-xs text-gray-500">PID {alert.processId}</span>
                </div>
                <p className="text-xs text-gray-400 line-clamp-2">
                  {alert.description}
                </p>
                {alert.detectorResults && alert.detectorResults.length > 0 && (
                  <div className="flex flex-wrap gap-1 mt-1.5">
                    {alert.detectorResults.filter(d => d.score > 0).map((d, j) => (
                      <span key={j} className="text-[10px] px-1.5 py-0.5 rounded bg-gray-700 text-gray-300">
                        {d.detectorName} ({(d.score * 100).toFixed(0)}%)
                      </span>
                    ))}
                  </div>
                )}
              </div>
              <div className="text-right shrink-0">
                <div className="text-lg font-bold text-red-400">
                  {(alert.riskScore * 100).toFixed(0)}%
                </div>
                <div className="text-[10px] text-gray-600">
                  {formatTime(alert.timestampNs)}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
