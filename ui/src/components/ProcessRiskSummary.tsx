import type { ProcessRiskEntry } from '../types';
import { Activity } from 'lucide-react';

interface Props {
  processes: ProcessRiskEntry[];
}

function riskColor(score: number): string {
  if (score >= 0.75) return 'var(--accent-red)';
  if (score >= 0.5) return 'var(--accent-orange)';
  if (score >= 0.25) return 'var(--accent-cyan)';
  return 'var(--accent-green)';
}

function riskGradient(score: number): string {
  if (score >= 0.75) return 'linear-gradient(90deg, #ef4444, #f87171)';
  if (score >= 0.5) return 'linear-gradient(90deg, #f59e0b, #fbbf24)';
  if (score >= 0.25) return 'linear-gradient(90deg, #06b6d4, #22d3ee)';
  return 'linear-gradient(90deg, #22c55e, #4ade80)';
}

export default function ProcessRiskSummary({ processes }: Props) {
  const sorted = [...processes]
    .sort((a, b) => b.currentRiskScore - a.currentRiskScore)
    .slice(0, 5);

  return (
    <div className="neu-flat p-5" style={{ minHeight: 340 }}>
      <div className="flex items-center justify-between mb-5">
        <div className="flex items-center gap-2">
          <Activity size={14} style={{ color: 'var(--accent-blue)' }} />
          <span className="card-title">Top Process Risks</span>
        </div>
        <span className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
          {processes.length} tracked
        </span>
      </div>

      {sorted.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12" style={{ color: 'var(--text-muted)' }}>
          <Activity size={32} className="mb-3 opacity-30" />
          <p className="text-sm font-medium opacity-60">No processes tracked</p>
        </div>
      ) : (
        <div className="space-y-3">
          {sorted.map((proc, idx) => (
            <div
              key={proc.processId}
              className="neu-pressed-sm p-3.5 flex items-center gap-4 cursor-default"
              style={{
                animationDelay: `${idx * 60}ms`,
                animation: 'fade-in 0.4s ease-out both',
              }}
            >
              {/* Rank */}
              <div
                className="text-xs font-bold font-mono shrink-0 flex items-center justify-center"
                style={{
                  width: 28,
                  height: 28,
                  borderRadius: 'var(--radius-sm)',
                  background: 'var(--bg-surface)',
                  boxShadow: '2px 2px 5px var(--shadow-dark), -2px -2px 5px var(--shadow-light)',
                  color: 'var(--text-muted)',
                }}
              >
                #{idx + 1}
              </div>

              {/* Info */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1.5">
                  <span className="text-sm font-semibold truncate" style={{ color: 'var(--text-primary)' }}>
                    {shortenProcessName(proc.processName)}
                  </span>
                  <span className="text-[10px] font-mono" style={{ color: 'var(--text-muted)' }}>
                    PID {proc.processId}
                  </span>
                  {proc.isQuarantined && <span className="badge badge-quarantined">Quarantined</span>}
                </div>
                <div className="risk-bar-track">
                  <div
                    className="risk-bar-fill"
                    style={{
                      width: `${Math.max(proc.currentRiskScore * 100, 2)}%`,
                      background: riskGradient(proc.currentRiskScore),
                      boxShadow: `0 0 6px ${riskColor(proc.currentRiskScore)}40`,
                    }}
                  />
                </div>
              </div>

              {/* Score */}
              <div className="text-right shrink-0">
                <div
                  className="text-lg font-bold font-mono"
                  style={{ color: riskColor(proc.currentRiskScore) }}
                >
                  {(proc.currentRiskScore * 100).toFixed(0)}%
                </div>
                <div className="text-[10px] font-mono" style={{ color: 'var(--text-muted)' }}>
                  {proc.eventCount} evt
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function shortenProcessName(name: string): string {
  if (!name) return 'Unknown';
  const parts = name.replace(/\\/g, '/').split('/');
  return parts[parts.length - 1] || name;
}
