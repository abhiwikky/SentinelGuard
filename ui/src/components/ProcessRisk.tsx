import { useState } from 'react';
import type { ProcessRiskEntry, DetectorResult } from '../types';

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

function riskBarGradient(score: number): string {
  if (score >= 0.75) return 'bg-gradient-to-r from-red-600 to-red-400';
  if (score >= 0.5) return 'bg-gradient-to-r from-orange-600 to-orange-400';
  if (score >= 0.25) return 'bg-gradient-to-r from-yellow-600 to-yellow-400';
  return 'bg-gradient-to-r from-green-600 to-green-400';
}

function detectorDisplayName(name: string): string {
  return name
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

function detectorIcon(name: string): string {
  const icons: Record<string, string> = {
    entropy_spike: '🔥',
    mass_write: '📝',
    mass_rename_delete: '🔄',
    ransom_note: '📄',
    shadow_copy: '👻',
    process_behavior: '⚙️',
    extension_explosion: '💥',
  };
  return icons[name] || '🔍';
}

export default function ProcessRisk({ processes }: Props) {
  const [expandedPids, setExpandedPids] = useState<Set<number>>(new Set());

  const sorted = [...processes].sort((a, b) => b.currentRiskScore - a.currentRiskScore);

  const toggleExpanded = (pid: number) => {
    setExpandedPids((prev) => {
      const next = new Set(prev);
      if (next.has(pid)) {
        next.delete(pid);
      } else {
        next.add(pid);
      }
      return next;
    });
  };

  const activeDetectors = (results: DetectorResult[] | undefined): DetectorResult[] => {
    if (!results) return [];
    return [...results]
      .filter((r) => r.score > 0)
      .sort((a, b) => b.score - a.score);
  };

  return (
    <div className="card animate-fade-in">
      <div className="card-header">Process Risk Overview</div>

      {sorted.length === 0 ? (
        <div className="text-center py-6 text-gray-600 text-sm">
          No processes tracked
        </div>
      ) : (
        <div className="space-y-2 max-h-[32rem] overflow-y-auto pr-1">
          {sorted.map((proc) => {
            const isExpanded = expandedPids.has(proc.processId);
            const detectors = activeDetectors(proc.detectorResults);

            return (
              <div
                key={proc.processId}
                className="rounded-lg bg-gray-800/40 transition-colors overflow-hidden"
              >
                {/* Clickable header row */}
                <button
                  type="button"
                  onClick={() => toggleExpanded(proc.processId)}
                  className={`w-full flex items-center gap-3 p-2.5 text-left transition-colors hover:bg-gray-800/60 cursor-pointer ${isExpanded ? 'bg-gray-800/60' : ''}`}
                  id={`process-risk-row-${proc.processId}`}
                >
                  {/* Expand chevron */}
                  <div
                    className={`flex-shrink-0 w-4 h-4 flex items-center justify-center transition-transform duration-300 text-gray-400 ${
                      isExpanded ? 'rotate-90' : ''
                    }`}
                  >
                    <svg
                      width="10"
                      height="10"
                      viewBox="0 0 10 10"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    >
                      <path d="M3 1 L7 5 L3 9" />
                    </svg>
                  </div>

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
                </button>

                {/* Expandable detail section */}
                <div
                  className={`risk-details-panel ${isExpanded ? 'risk-details-panel--open' : ''}`}
                >
                  <div className="px-3 pb-3 pt-1">
                    {/* Detector Breakdown */}
                    {detectors.length > 0 ? (
                      <div className="mb-3">
                        <div className="text-[11px] uppercase tracking-wider text-gray-500 mb-2 font-semibold">
                          Risk Detectors
                        </div>
                        <div className="space-y-2">
                          {detectors.map((det) => (
                            <div
                              key={det.detectorName}
                              className="bg-gray-900/60 rounded-lg p-2.5 border border-gray-700/50"
                            >
                              <div className="flex items-center gap-2 mb-1.5">
                                <span className="text-sm">{detectorIcon(det.detectorName)}</span>
                                <span className="text-xs font-medium text-gray-200">
                                  {detectorDisplayName(det.detectorName)}
                                </span>
                                <span className={`ml-auto text-xs font-mono font-bold ${riskColor(det.score)}`}>
                                  {(det.score * 100).toFixed(0)}%
                                </span>
                              </div>

                              {/* Detector score bar */}
                              <div className="h-1 bg-gray-700 rounded-full overflow-hidden mb-1.5">
                                <div
                                  className={`h-full rounded-full transition-all duration-500 ${riskBarGradient(det.score)}`}
                                  style={{ width: `${Math.max(det.score * 100, 2)}%` }}
                                />
                              </div>

                              {/* Evidence items */}
                              {det.evidence && det.evidence.length > 0 && (
                                <div className="mt-1.5">
                                  {det.evidence.slice(0, 5).map((ev, idx) => (
                                    <div
                                      key={idx}
                                      className="text-[11px] text-gray-400 pl-2 py-0.5 border-l border-gray-700 mb-0.5 truncate"
                                      title={ev}
                                    >
                                      {ev}
                                    </div>
                                  ))}
                                  {det.evidence.length > 5 && (
                                    <div className="text-[10px] text-gray-600 pl-2 mt-0.5">
                                      +{det.evidence.length - 5} more
                                    </div>
                                  )}
                                </div>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                    ) : (
                      <div className="text-center py-3 text-gray-500 text-xs mb-3">
                        No detector details available yet
                      </div>
                    )}

                    {/* Score Composition */}
                    <div className="bg-gray-900/40 rounded-lg p-2.5 border border-gray-700/30">
                      <div className="text-[11px] uppercase tracking-wider text-gray-500 mb-2 font-semibold">
                        Score Composition
                      </div>
                      <div className="grid grid-cols-3 gap-3 text-center">
                        <div>
                          <div className={`text-sm font-mono font-bold ${riskColor(proc.weightedScore || 0)}`}>
                            {((proc.weightedScore || 0) * 100).toFixed(0)}%
                          </div>
                          <div className="text-[10px] text-gray-500 mt-0.5">Weighted</div>
                        </div>
                        <div>
                          <div className={`text-sm font-mono font-bold ${riskColor(proc.mlScore || 0)}`}>
                            {((proc.mlScore || 0) * 100).toFixed(0)}%
                          </div>
                          <div className="text-[10px] text-gray-500 mt-0.5">ML Model</div>
                        </div>
                        <div>
                          <div className={`text-sm font-mono font-bold ${riskColor(proc.currentRiskScore)}`}>
                            {(proc.currentRiskScore * 100).toFixed(0)}%
                          </div>
                          <div className="text-[10px] text-gray-500 mt-0.5">Final</div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
