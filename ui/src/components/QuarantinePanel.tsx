import { useState } from 'react';
import type { QuarantinedProcess } from '../types';

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
    <div className="card animate-fade-in">
      <div className="flex items-center justify-between mb-3">
        <div className="card-header mb-0">Quarantined Processes</div>
        {processes.length > 0 && (
          <span className="badge badge-critical">
            {processes.length} active
          </span>
        )}
      </div>

      {processes.length === 0 ? (
        <div className="text-center py-6 text-gray-600 text-sm">
          <svg className="w-8 h-8 mx-auto mb-2 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
          </svg>
          No quarantined processes
        </div>
      ) : (
        <div className="space-y-2">
          {processes.map((proc) => (
            <div
              key={proc.processId}
              className="flex items-center justify-between p-3 rounded-lg bg-red-900/10 border border-red-900/30"
            >
              <div>
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-white">
                    {proc.processName || 'Unknown'}
                  </span>
                  <span className="text-xs text-gray-500">PID {proc.processId}</span>
                </div>
                <div className="text-xs text-gray-500 mt-0.5">
                  Risk: {(proc.riskScore * 100).toFixed(0)}% · Quarantined: {formatTime(proc.quarantinedAtNs)}
                </div>
              </div>
              <button
                className="btn btn-outline text-xs"
                onClick={() => handleRelease(proc.processId)}
                disabled={releasing === proc.processId}
              >
                {releasing === proc.processId ? 'Releasing...' : 'Release'}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
