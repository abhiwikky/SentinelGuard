import type { ProcessRiskEntry } from '../types';

interface Props {
  processes: ProcessRiskEntry[];
}

function riskColor(score: number): string {
  if (score >= 0.75) return 'text-red-400';
  if (score >= 0.5) return 'text-orange-400';
  if (score >= 0.25) return 'text-yellow-400';
  return 'text-green-400';
}

function riskBar(score: number): string {
  if (score >= 0.75) return 'bg-red-500';
  if (score >= 0.5) return 'bg-orange-500';
  if (score >= 0.25) return 'bg-yellow-500';
  return 'bg-green-500';
}

export default function ProcessRisk({ processes }: Props) {
  const sorted = [...processes].sort((a, b) => b.currentRiskScore - a.currentRiskScore);

  return (
    <div className="card animate-fade-in">
      <div className="card-header">Process Risk Overview</div>

      {sorted.length === 0 ? (
        <div className="text-center py-6 text-gray-600 text-sm">
          No processes tracked
        </div>
      ) : (
        <div className="space-y-2 max-h-80 overflow-y-auto">
          {sorted.map((proc) => (
            <div
              key={proc.processId}
              className="flex items-center gap-3 p-2.5 rounded-lg bg-gray-800/40 hover:bg-gray-800/60 transition-colors"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-white truncate">
                    {proc.processName || 'Unknown'}
                  </span>
                  <span className="text-[10px] text-gray-500">PID {proc.processId}</span>
                  {proc.isQuarantined && (
                    <span className="badge badge-critical text-[10px]">Quarantined</span>
                  )}
                </div>
                <div className="flex items-center gap-2 mt-1">
                  <div className="flex-1 h-1.5 bg-gray-700 rounded-full overflow-hidden">
                    <div
                      className={`h-full rounded-full transition-all duration-500 ${riskBar(proc.currentRiskScore)}`}
                      style={{ width: `${Math.max(proc.currentRiskScore * 100, 2)}%` }}
                    />
                  </div>
                  <span className={`text-xs font-mono font-bold ${riskColor(proc.currentRiskScore)}`}>
                    {(proc.currentRiskScore * 100).toFixed(0)}%
                  </span>
                </div>
              </div>
              <div className="text-right text-xs text-gray-500">
                {proc.eventCount} events
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
