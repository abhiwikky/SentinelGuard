import { useState } from 'react';
import { useAppData } from '../App';
import type { ProcessRiskEntry, DetectorResult } from '../types';
import {
  Activity,
  ChevronDown,
  ChevronUp,
  Cpu,
  FlaskConical,
  Target,
  FileWarning,
  Eye,
  Info,
} from 'lucide-react';

// All 7 known detectors
const DETECTORS = [
  'entropy_spike',
  'mass_write',
  'mass_rename_delete',
  'ransom_note',
  'shadow_copy',
  'process_behavior',
  'extension_explosion',
];

const DETECTOR_DESCRIPTIONS: Record<string, string> = {
  entropy_spike: 'Measures file content randomness. High entropy suggests encryption — the hallmark of ransomware encrypting files. Score is based on how many high-entropy writes were observed compared to the threshold.',
  mass_write: 'Tracks rapid bulk file write operations. Ransomware typically writes encrypted content to many files very quickly. Score scales with the ratio of writes per second vs normal baseline.',
  mass_rename_delete: 'Detects bulk renaming or deletion of files. Ransomware often renames files with new extensions (.encrypted, .locked) or deletes originals after encryption.',
  ransom_note: 'Scans for creation of ransom note files (README.txt, DECRYPT_FILES.html, etc.) using known filename patterns. Any match triggers a high score.',
  shadow_copy: 'Monitors for Volume Shadow Copy deletion attempts (vssadmin, wmic shadowcopy). Ransomware deletes shadow copies to prevent file recovery.',
  process_behavior: 'Analyzes unique extensions written and directory traversal breadth. Processes touching too many unique extensions across too many directories indicate indiscriminate file modification.',
  extension_explosion: 'Detects a sudden spike in unique file extensions being written to, which indicates a process is appending novel extensions — a strong ransomware indicator.',
};

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

function detectorDisplayName(name: string): string {
  return name.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
}

function shortenProcessName(name: string): string {
  if (!name) return 'Unknown';
  const parts = name.replace(/\\/g, '/').split('/');
  return parts[parts.length - 1] || name;
}

export default function ProcessRiskPage() {
  const { processes } = useAppData();
  const [expandedPid, setExpandedPid] = useState<number | null>(null);
  const [detectorInfoId, setDetectorInfoId] = useState<string | null>(null);

  const sorted = [...processes].sort((a, b) => b.currentRiskScore - a.currentRiskScore);

  const toggleExpanded = (pid: number) => {
    setExpandedPid(expandedPid === pid ? null : pid);
    setDetectorInfoId(null);
  };

  const getUniformDetectors = (results: DetectorResult[] | undefined) => {
    const resMap = new Map((results || []).map((r) => [r.detectorName, r]));
    return DETECTORS.map((name) => {
      const existing = resMap.get(name);
      return (
        existing || {
          detectorName: name,
          score: 0,
          evidence: [],
          timestampNs: '0',
          processId: 0,
        }
      );
    });
  };

  return (
    <div className="animate-fade-in max-w-6xl">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div className="neu-pressed-sm flex items-center justify-center" style={{ width: 36, height: 36 }}>
            <Activity size={18} style={{ color: 'var(--accent-blue)' }} />
          </div>
          <div>
            <h2 className="text-base font-bold" style={{ color: 'var(--text-primary)', margin: 0 }}>
              Process Risk Overview
            </h2>
            <p className="text-xs" style={{ color: 'var(--text-muted)', margin: 0 }}>
              Detailed risk analysis with detector-level explanations
            </p>
          </div>
        </div>
        <div className="neu-flat-sm px-3 py-1.5">
          <span className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
            {processes.length} processes tracked
          </span>
        </div>
      </div>

      {/* Process List */}
      {sorted.length === 0 ? (
        <div className="neu-flat p-12 flex flex-col items-center justify-center" style={{ color: 'var(--text-muted)' }}>
          <Activity size={40} className="mb-4 opacity-20" />
          <p className="text-sm font-medium">No processes currently being tracked</p>
          <p className="text-xs mt-1 opacity-60">Processes will appear when the agent detects file activity</p>
        </div>
      ) : (
        <div className="space-y-3">
          {sorted.map((proc, idx) => {
            const isExpanded = expandedPid === proc.processId;
            const uniformDetectors = getUniformDetectors(proc.detectorResults);
            const activeDetectors = uniformDetectors.filter((d) => d.score > 0);
            const processLogs = (proc.detectorResults || [])
              .filter((r) => r.score > 0 && r.evidence && r.evidence.length > 0)
              .flatMap((r) => r.evidence.map((ev) => ({ detector: r.detectorName, evidence: ev })));

            return (
              <div
                key={proc.processId}
                className="animate-fade-in"
                style={{ animationDelay: `${idx * 40}ms` }}
              >
                {/* Process Row */}
                <button
                  type="button"
                  onClick={() => toggleExpanded(proc.processId)}
                  className="w-full text-left neu-convex p-4 transition-all duration-200 hover:translate-y-[-1px] cursor-pointer"
                  style={{
                    borderRadius: isExpanded ? 'var(--radius-md) var(--radius-md) 0 0' : 'var(--radius-md)',
                  }}
                >
                  <div className="flex items-center gap-4">
                    {/* Risk Indicator */}
                    <div
                      className="shrink-0 flex items-center justify-center font-mono font-bold text-sm"
                      style={{
                        width: 48,
                        height: 48,
                        borderRadius: 'var(--radius-sm)',
                        background: 'var(--bg-inset)',
                        boxShadow: 'inset 2px 2px 4px var(--shadow-dark), inset -2px -2px 4px var(--shadow-light)',
                        color: riskColor(proc.currentRiskScore),
                        textShadow: proc.currentRiskScore >= 0.5 ? `0 0 10px ${riskColor(proc.currentRiskScore)}50` : 'none',
                      }}
                    >
                      {(proc.currentRiskScore * 100).toFixed(0)}%
                    </div>

                    {/* Process Info */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-semibold text-sm truncate" style={{ color: 'var(--text-primary)' }}>
                          {proc.processName || 'Unknown'}
                        </span>
                        <span className="text-[10px] font-mono" style={{ color: 'var(--text-muted)' }}>
                          PID {proc.processId}
                        </span>
                        {proc.isQuarantined && <span className="badge badge-quarantined">Quarantined</span>}
                      </div>
                      <div className="flex items-center gap-4">
                        <div className="flex-1">
                          <div className="risk-bar-track">
                            <div
                              className="risk-bar-fill"
                              style={{
                                width: `${Math.max(proc.currentRiskScore * 100, 2)}%`,
                                background: riskGradient(proc.currentRiskScore),
                                boxShadow: `0 0 6px ${riskColor(proc.currentRiskScore)}30`,
                              }}
                            />
                          </div>
                        </div>
                        <span className="text-[10px] font-mono shrink-0" style={{ color: 'var(--text-muted)' }}>
                          {proc.eventCount} events
                        </span>
                      </div>
                      {/* Inline detector tags */}
                      {activeDetectors.length > 0 && (
                        <div className="flex flex-wrap gap-1.5 mt-2">
                          {activeDetectors.map((d) => (
                            <span
                              key={d.detectorName}
                              className="text-[9px] font-bold uppercase px-1.5 py-0.5 rounded"
                              style={{
                                background: 'rgba(239,68,68,0.1)',
                                color: riskColor(d.score),
                                letterSpacing: '0.04em',
                              }}
                            >
                              {detectorDisplayName(d.detectorName)} {(d.score * 100).toFixed(0)}%
                            </span>
                          ))}
                        </div>
                      )}
                    </div>

                    {/* Expand icon */}
                    <div className="shrink-0" style={{ color: 'var(--text-muted)' }}>
                      {isExpanded ? <ChevronUp size={18} /> : <ChevronDown size={18} />}
                    </div>
                  </div>
                </button>

                {/* Expanded Detail Panel */}
                <div className="expandable-panel" data-expanded={isExpanded}>
                  <div className="expandable-content">
                    <div
                      className="p-5 space-y-5"
                      style={{
                        background: 'var(--bg-inset)',
                        borderRadius: '0 0 var(--radius-md) var(--radius-md)',
                        borderTop: '1px solid rgba(255,255,255,0.03)',
                      }}
                    >
                      {/* Score Composition */}
                      <div>
                        <div className="flex items-center gap-2 mb-3">
                          <Target size={13} style={{ color: 'var(--accent-blue)' }} />
                          <span className="card-title">Score Composition</span>
                        </div>
                        <div className="grid grid-cols-3 gap-3">
                          <ScoreCard
                            label="Heuristic Score"
                            value={proc.weightedScore || 0}
                            icon={<FlaskConical size={14} />}
                            description="Weighted aggregate of all detector scores"
                          />
                          <ScoreCard
                            label="ML Model Score"
                            value={proc.mlScore || 0}
                            icon={<Cpu size={14} />}
                            description="Neural network classification confidence"
                          />
                          <ScoreCard
                            label="Final Risk"
                            value={proc.currentRiskScore}
                            icon={<Target size={14} />}
                            description="Combined heuristic + ML with correlation boost"
                            highlight
                          />
                        </div>
                      </div>

                      {/* Detector Matrix */}
                      <div>
                        <div className="flex items-center gap-2 mb-3">
                          <Eye size={13} style={{ color: 'var(--accent-blue)' }} />
                          <span className="card-title">Detector Analysis</span>
                          <span className="text-[10px] ml-1" style={{ color: 'var(--text-muted)' }}>
                            Click any detector for explanation
                          </span>
                        </div>
                        <div className="grid grid-cols-2 md:grid-cols-4 gap-2.5">
                          {uniformDetectors.map((det) => {
                            const isActive = det.score > 0;
                            const isInfoOpen = detectorInfoId === `${proc.processId}-${det.detectorName}`;

                            return (
                              <button
                                key={det.detectorName}
                                type="button"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  setDetectorInfoId(isInfoOpen ? null : `${proc.processId}-${det.detectorName}`);
                                }}
                                className={`detector-pill cursor-pointer text-left transition-all duration-200 ${isActive ? 'active' : ''}`}
                                style={{
                                  borderColor: isInfoOpen ? 'var(--accent-blue)' : undefined,
                                  border: isInfoOpen ? '1px solid var(--accent-blue)' : undefined,
                                }}
                              >
                                <div className="flex items-center justify-between">
                                  <span className="text-[9px] font-bold uppercase" style={{ color: isActive ? 'var(--text-primary)' : 'var(--text-muted)' }}>
                                    {detectorDisplayName(det.detectorName)}
                                  </span>
                                  <Info size={10} style={{ color: 'var(--text-muted)', opacity: 0.5 }} />
                                </div>
                                <div
                                  className="text-sm font-mono font-bold"
                                  style={{ color: isActive ? riskColor(det.score) : 'var(--text-muted)', opacity: isActive ? 1 : 0.4 }}
                                >
                                  {isActive ? `${(det.score * 100).toFixed(0)}%` : '0%'}
                                </div>

                                {/* Info tooltip */}
                                {isInfoOpen && (
                                  <div
                                    className="mt-2 pt-2 text-[10px] leading-relaxed"
                                    style={{ color: 'var(--text-secondary)', borderTop: '1px solid rgba(255,255,255,0.06)' }}
                                  >
                                    {DETECTOR_DESCRIPTIONS[det.detectorName] || 'No description available.'}
                                  </div>
                                )}
                              </button>
                            );
                          })}
                        </div>
                      </div>

                      {/* Process Behavioral Logs */}
                      <div>
                        <div className="flex items-center gap-2 mb-3">
                          <FileWarning size={13} style={{ color: 'var(--accent-orange)' }} />
                          <span className="card-title">Behavioral Evidence</span>
                          {processLogs.length > 0 && (
                            <span className="text-[10px] font-mono ml-1" style={{ color: 'var(--text-muted)' }}>
                              {processLogs.length} entries
                            </span>
                          )}
                        </div>
                        <div
                          className="neu-pressed p-4 max-h-52 overflow-y-auto custom-scroll font-mono text-[11px] leading-relaxed"
                          style={{ borderRadius: 'var(--radius-sm)' }}
                        >
                          {processLogs.length === 0 ? (
                            <span style={{ color: 'var(--text-muted)', opacity: 0.5 }}>
                              No malicious behavior logged for this process.
                            </span>
                          ) : (
                            <div className="space-y-1.5">
                              {processLogs.map((log, logIdx) => (
                                <div key={logIdx} className="flex gap-2">
                                  <span
                                    className="shrink-0"
                                    style={{ color: riskColor(0.8), opacity: 0.8 }}
                                  >
                                    [{log.detector}]
                                  </span>
                                  <span style={{ color: 'var(--text-secondary)', wordBreak: 'break-all' }}>
                                    {log.evidence}
                                  </span>
                                </div>
                              ))}
                            </div>
                          )}
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

// ─── Score Composition Card ───
function ScoreCard({
  label,
  value,
  icon,
  description,
  highlight,
}: {
  label: string;
  value: number;
  icon: React.ReactNode;
  description: string;
  highlight?: boolean;
}) {
  return (
    <div
      className={highlight ? 'neu-pressed p-3.5' : 'neu-flat-sm p-3.5'}
      style={{
        border: highlight ? `1px solid ${riskColor(value)}20` : undefined,
      }}
    >
      <div className="flex items-center gap-1.5 mb-2">
        <span style={{ color: 'var(--accent-blue)' }}>{icon}</span>
        <span className="text-[10px] font-bold uppercase" style={{ color: 'var(--text-muted)' }}>
          {label}
        </span>
      </div>
      <div
        className="text-xl font-bold font-mono"
        style={{
          color: riskColor(value),
          textShadow: highlight ? `0 0 10px ${riskColor(value)}40` : 'none',
        }}
      >
        {(value * 100).toFixed(0)}%
      </div>
      <p className="text-[9px] mt-1.5 leading-tight" style={{ color: 'var(--text-muted)' }}>
        {description}
      </p>
    </div>
  );
}
