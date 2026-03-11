import React from 'react';
import { useDashboardData, useDetectorLogsData } from '../context/DashboardDataContext';

export const DetectorLogs: React.FC = () => {
  const entries = useDetectorLogsData().entries || [];
  const { snapshot } = useDashboardData();
  const error = snapshot?.errors.detectorLogs || null;

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

