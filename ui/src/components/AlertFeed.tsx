import { useState } from 'react';
import type { Alert } from '../types';
import { Bell, ChevronDown, ChevronUp, Clock } from 'lucide-react';

interface Props {
  alerts: Alert[];
  compact?: boolean;
}

function severityLabel(severity: string): string {
  const map: Record<string, string> = {
    SEVERITY_CRITICAL: 'CRITICAL', SEVERITY_HIGH: 'HIGH',
    SEVERITY_MEDIUM: 'MEDIUM', SEVERITY_LOW: 'LOW',
    '4': 'CRITICAL', '3': 'HIGH', '2': 'MEDIUM', '1': 'LOW',
  };
  return map[severity] || severity;
}

function severityBadgeClass(severity: string): string {
  const map: Record<string, string> = {
    SEVERITY_CRITICAL: 'badge-critical', SEVERITY_HIGH: 'badge-high',
    SEVERITY_MEDIUM: 'badge-medium', SEVERITY_LOW: 'badge-low',
    '4': 'badge-critical', '3': 'badge-high', '2': 'badge-medium', '1': 'badge-low',
  };
  return map[severity] || 'badge-low';
}

function formatTime(ns: string): string {
  const ms = parseInt(ns) / 1_000_000;
  if (isNaN(ms) || ms === 0) return '—';
  return new Date(ms).toLocaleString();
}

function formatTimeShort(ns: string): string {
  const ms = parseInt(ns) / 1_000_000;
  if (isNaN(ms) || ms === 0) return '—';
  return new Date(ms).toLocaleTimeString();
}

function shortenName(name: string): string {
  if (!name) return 'Unknown';
  const parts = name.replace(/\\/g, '/').split('/');
  return parts[parts.length - 1] || name;
}

export default function AlertFeed({ alerts, compact }: Props) {
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const displayAlerts = compact ? alerts.slice(0, 5) : alerts;

  return (
    <div className="neu-flat p-5 h-full flex flex-col" style={{ minHeight: compact ? 340 : undefined }}>
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Bell size={14} style={{ color: 'var(--accent-red)' }} />
          <span className="card-title">Alert Feed</span>
        </div>
        {alerts.length > 0 && (
          <div className="flex items-center gap-1.5">
            <div className="status-dot error" style={{ width: 6, height: 6 }} />
            <span className="text-[10px] font-bold uppercase" style={{ color: 'var(--accent-red)' }}>
              Live
            </span>
          </div>
        )}
      </div>

      {/* Alert List */}
      <div className="flex-1 overflow-y-auto custom-scroll pr-1 space-y-2.5">
        {displayAlerts.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-10" style={{ color: 'var(--text-muted)' }}>
            <Bell size={28} className="mb-3 opacity-20" />
            <p className="text-sm font-medium opacity-60">No alerts — system is clean</p>
          </div>
        ) : (
          displayAlerts.map((alert, idx) => {
            const isExpanded = expandedId === alert.alertId && !compact;

            return (
              <div
                key={alert.alertId}
                className="animate-fade-in"
                style={{ animationDelay: `${idx * 30}ms` }}
              >
                <button
                  type="button"
                  onClick={() => !compact && setExpandedId(isExpanded ? null : alert.alertId)}
                  className={`w-full text-left p-3 transition-all duration-200 ${
                    compact ? 'neu-pressed-sm cursor-default' : isExpanded ? 'neu-pressed cursor-pointer' : 'neu-flat-sm cursor-pointer hover:translate-y-[-1px]'
                  }`}
                >
                  <div className="flex items-start gap-3">
                    <span className={`badge ${severityBadgeClass(alert.severity)} mt-0.5 shrink-0`}>
                      {severityLabel(alert.severity)}
                    </span>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-semibold truncate" style={{ color: 'var(--text-primary)' }}>
                          {shortenName(alert.processName)}
                        </span>
                        <span className="text-[10px] font-mono" style={{ color: 'var(--text-muted)' }}>
                          PID {alert.processId}
                        </span>
                      </div>
                      <p
                        className="text-xs mt-0.5"
                        style={{
                          color: 'var(--text-secondary)',
                          display: '-webkit-box',
                          WebkitLineClamp: isExpanded ? 999 : 1,
                          WebkitBoxOrient: 'vertical',
                          overflow: 'hidden',
                        }}
                      >
                        {alert.description}
                      </p>
                    </div>
                    <div className="text-right shrink-0">
                      <div className="text-base font-bold font-mono" style={{ color: 'var(--accent-red)' }}>
                        {(alert.riskScore * 100).toFixed(0)}%
                      </div>
                      <div className="flex items-center gap-1 justify-end mt-0.5">
                        <Clock size={9} style={{ color: 'var(--text-muted)' }} />
                        <span className="text-[9px] font-mono" style={{ color: 'var(--text-muted)' }}>
                          {compact ? formatTimeShort(alert.timestampNs) : formatTime(alert.timestampNs)}
                        </span>
                      </div>
                    </div>
                  </div>
                </button>

                {/* Expanded Details */}
                {!compact && (
                  <div className="expandable-panel" data-expanded={isExpanded}>
                    <div className="expandable-content">
                      <div
                        className="p-4 mt-1 space-y-3"
                        style={{
                          background: 'var(--bg-inset)',
                          borderRadius: 'var(--radius-sm)',
                          border: '1px solid rgba(255,255,255,0.03)',
                        }}
                      >
                        {/* Context */}
                        <div className="grid grid-cols-2 gap-3 text-xs">
                          <InfoRow label="Alert ID" value={alert.alertId} mono />
                          <InfoRow label="Process" value={alert.processName} mono />
                          <InfoRow label="PID" value={String(alert.processId)} mono />
                          <InfoRow
                            label="Quarantine"
                            value={alert.quarantineStatus.replace('QS_', '')}
                            color="var(--accent-orange)"
                          />
                        </div>

                        {/* Triggered Detectors */}
                        {alert.detectorResults && alert.detectorResults.filter((d) => d.score > 0).length > 0 && (
                          <div>
                            <span className="card-title text-[10px]">Triggered Detectors</span>
                            <div className="grid grid-cols-2 gap-2 mt-2">
                              {alert.detectorResults
                                .filter((d) => d.score > 0)
                                .map((d, j) => (
                                  <div
                                    key={j}
                                    className="detector-pill active flex-row items-center justify-between"
                                    style={{ flexDirection: 'row', padding: '0.4rem 0.6rem' }}
                                  >
                                    <span className="text-[10px] font-bold uppercase" style={{ color: 'var(--text-primary)' }}>
                                      {d.detectorName.replace(/_/g, ' ')}
                                    </span>
                                    <span className="text-xs font-mono font-bold" style={{ color: 'var(--accent-red)' }}>
                                      {(d.score * 100).toFixed(0)}%
                                    </span>
                                  </div>
                                ))}
                            </div>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

function InfoRow({ label, value, mono, color }: { label: string; value: string; mono?: boolean; color?: string }) {
  return (
    <div className="flex justify-between items-baseline">
      <span style={{ color: 'var(--text-muted)' }}>{label}</span>
      <span
        className={`${mono ? 'font-mono' : ''} truncate ml-2`}
        style={{ color: color || 'var(--text-secondary)', maxWidth: '60%', textAlign: 'right' }}
      >
        {value}
      </span>
    </div>
  );
}
