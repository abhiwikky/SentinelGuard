import type { DetectorResult } from '../types';

interface Props {
  results: DetectorResult[];
}

function scoreColor(score: number): string {
  if (score >= 0.7) return 'text-red-400';
  if (score >= 0.4) return 'text-orange-400';
  if (score >= 0.1) return 'text-yellow-400';
  return 'text-gray-500';
}

function detectorIcon(name: string): string {
  const icons: Record<string, string> = {
    entropy_spike: '🔐',
    mass_write: '📝',
    mass_rename_delete: '📂',
    ransom_note: '📄',
    shadow_copy: '🗑️',
    process_behavior: '⚙️',
    extension_explosion: '💥',
  };
  return icons[name] || '🔍';
}

function formatTime(ns: string): string {
  const ms = parseInt(ns) / 1_000_000;
  if (isNaN(ms) || ms === 0) return '—';
  return new Date(ms).toLocaleTimeString();
}

export default function DetectorLogs({ results }: Props) {
  // Filter to only show results with non-zero scores
  const significant = results.filter((r) => r.score > 0);

  return (
    <div className="card animate-fade-in">
      <div className="flex items-center justify-between mb-3">
        <div className="card-header mb-0">Detector Activity</div>
        <span className="text-xs text-gray-600">
          {significant.length} significant detections
        </span>
      </div>

      {significant.length === 0 ? (
        <div className="text-center py-6 text-gray-600 text-sm">
          No significant detector activity
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-gray-500 uppercase border-b border-gray-800">
                <th className="pb-2 pr-4">Detector</th>
                <th className="pb-2 pr-4">Score</th>
                <th className="pb-2 pr-4">PID</th>
                <th className="pb-2 pr-4">Evidence</th>
                <th className="pb-2">Time</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-800/50">
              {significant.slice(0, 50).map((result, i) => (
                <tr key={i} className="hover:bg-gray-800/30">
                  <td className="py-2 pr-4">
                    <span className="mr-1.5">{detectorIcon(result.detectorName)}</span>
                    <span className="font-medium text-gray-300">
                      {result.detectorName.replace(/_/g, ' ')}
                    </span>
                  </td>
                  <td className="py-2 pr-4">
                    <span className={`font-mono font-bold ${scoreColor(result.score)}`}>
                      {(result.score * 100).toFixed(0)}%
                    </span>
                  </td>
                  <td className="py-2 pr-4 text-gray-500">{result.processId}</td>
                  <td className="py-2 pr-4 text-xs text-gray-500 max-w-xs truncate">
                    {result.evidence[0] || '—'}
                  </td>
                  <td className="py-2 text-xs text-gray-600">
                    {formatTime(result.timestampNs)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
