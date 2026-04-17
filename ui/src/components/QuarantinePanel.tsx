import { useState } from 'react';
import type { QuarantinedProcess } from '../types';
import { ShieldAlert, Unlock, Clock } from 'lucide-react';

interface Props {
  processes: QuarantinedProcess[];
  onRelease: (processId: number) => void;
}

function formatTime(ns: string): string {
  const ms = parseInt(ns) / 1_000_000;
  if (isNaN(ms) || ms === 0) return '—';
  return new Date(ms).toLocaleString();
}

export default function QuarantinePanel({ processes, onRelease }: Props) {
  const [releasing, setReleasing] = useState<number | null>(null);

  const handleRelease = async (pid: number) => {
    setReleasing(pid);
    try {
      await onRelease(pid);
    } finally {
      setReleasing(null);
    }
  };

  return (
    <div className="neu-flat p-5">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <ShieldAlert size={14} style={{ color: 'var(--accent-purple)' }} />
          <span className="card-title">Quarantined Processes</span>
        </div>
        {processes.length > 0 && (
          <span className="badge badge-quarantined">{processes.length} held</span>
        )}
      </div>

      {processes.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-8" style={{ color: 'var(--text-muted)' }}>
          <ShieldAlert size={28} className="mb-3 opacity-20" />
          <p className="text-sm font-medium opacity-60">No quarantined processes</p>
          <p className="text-[10px] mt-1 opacity-40">Malicious processes will appear here when quarantined</p>
        </div>
      ) : (
        <div className="space-y-2.5">
          {processes.map((proc, idx) => (
            <div
              key={proc.processId}
              className="neu-pressed p-4 flex items-center justify-between gap-4 animate-fade-in"
              style={{ animationDelay: `${idx * 50}ms` }}
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-semibold text-sm" style={{ color: 'var(--accent-red)' }}>
                    {proc.processName || 'Unknown'}
                  </span>
                  <span className="text-[10px] font-mono" style={{ color: 'var(--text-muted)' }}>
                    PID {proc.processId}
                  </span>
                </div>
                <div className="flex items-center gap-4 text-[10px]" style={{ color: 'var(--text-muted)' }}>
                  <span className="font-mono">
                    Risk: <span style={{ color: 'var(--accent-red)' }}>{(proc.riskScore * 100).toFixed(0)}%</span>
                  </span>
                  <span className="flex items-center gap-1">
                    <Clock size={9} />
                    {formatTime(proc.quarantinedAtNs)}
                  </span>
                </div>
              </div>
              <button
                type="button"
                className="neu-button text-xs font-bold"
                style={{ color: 'var(--accent-blue)' }}
                onClick={() => handleRelease(proc.processId)}
                disabled={releasing === proc.processId}
              >
                <Unlock size={13} />
                {releasing === proc.processId ? 'Releasing...' : 'Release'}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
