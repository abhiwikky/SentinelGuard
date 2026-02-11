import React from 'react';
import { useEffect, useRef, useState } from 'react';
import { grpcClient, type DetectorLogEntry } from '../services/grpcClient';

export const DetectorLogs: React.FC = () => {
  const [entries, setEntries] = useState<DetectorLogEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const latestTimestampRef = useRef(0);

  useEffect(() => {
    let mounted = true;

    const refresh = async () => {
      const since = latestTimestampRef.current;
      try {
        const response = await grpcClient.getDetectorLogs(since, 100);
        if (!mounted) {
          return;
        }
        setError(null);
        const nextEntries = response.entries || [];
        if (nextEntries.length > 0) {
          latestTimestampRef.current = nextEntries[0].timestamp;
        }
        setEntries(nextEntries);
      } catch (err) {
        if (!mounted) {
          return;
        }
        setError(err instanceof Error ? err.message : 'Unable to fetch detector logs');
      }
    };

    void refresh();
    const interval = setInterval(() => {
      void refresh();
    }, 4000);

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-bold mb-4">Detector Logs</h2>
      {error && <p className="text-red-600 mb-3">{error}</p>}
      {entries.length === 0 ? (
        <p className="text-gray-600">No detector logs available.</p>
      ) : (
        <div className="space-y-2">
          {entries.map((entry, idx) => (
            <div key={`${entry.processId}-${entry.timestamp}-${idx}`} className="rounded border p-3">
              <p className="font-semibold">{entry.detectorName || 'Detector'}</p>
              <p className="text-sm text-gray-700">PID: {entry.processId}</p>
              <p className="text-sm text-gray-700">Score: {(entry.score * 100).toFixed(1)}%</p>
              <p className="text-sm text-gray-600">{entry.details || '-'}</p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

