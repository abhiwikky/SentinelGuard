import React from 'react';
import { useState } from 'react';
import { useDashboardData, useQuarantinedProcessesData } from '../context/DashboardDataContext';
import { grpcClient } from '../services/grpcClient';

export const QuarantinedProcesses: React.FC = () => {
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const processes = useQuarantinedProcessesData().processes || [];
  const { snapshot, refreshSnapshot } = useDashboardData();
  const error = snapshot?.errors.quarantinedProcesses || null;

  const handleRelease = async (processId: number) => {
    try {
      const response = await grpcClient.releaseFromQuarantine(processId);
      setActionMessage(response.message);
      await refreshSnapshot();
    } catch (err) {
      setActionMessage(err instanceof Error ? err.message : 'Failed to release process');
    }
  };

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-bold mb-4">Quarantined Processes</h2>
      {error && <p className="text-red-600 mb-3">{error}</p>}
      {actionMessage && <p className="text-blue-700 mb-3">{actionMessage}</p>}
      {processes.length === 0 ? (
        <p className="text-gray-600">No quarantined processes.</p>
      ) : (
        <div className="space-y-3">
          {processes.map((proc) => (
            <div key={`${proc.processId}-${proc.quarantinedAt}`} className="rounded border p-3">
              <p className="font-semibold">PID: {proc.processId}</p>
              <p className="text-sm text-gray-700">{proc.processPath || '-'}</p>
              <p className="text-sm text-gray-600">Reason: {proc.reason || 'N/A'}</p>
              <p className="text-sm text-gray-600">ML Score: {(proc.mlScore * 100).toFixed(1)}%</p>
              <button
                type="button"
                onClick={() => void handleRelease(proc.processId)}
                className="mt-2 rounded bg-blue-600 px-3 py-1.5 text-white hover:bg-blue-700"
              >
                Release
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

