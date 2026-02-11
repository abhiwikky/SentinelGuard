import React from 'react';
import { useEffect, useState } from 'react';
import { grpcClient, type ProcessRisk } from '../services/grpcClient';

export const ProcessRiskOverview: React.FC = () => {
  const [processes, setProcesses] = useState<ProcessRisk[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    const refresh = async () => {
      try {
        const response = await grpcClient.getProcessRiskOverview(50);
        if (!mounted) {
          return;
        }
        setError(null);
        setProcesses(response.processes || []);
      } catch (err) {
        if (!mounted) {
          return;
        }
        setError(err instanceof Error ? err.message : 'Unable to fetch process risks');
      }
    };

    void refresh();
    const interval = setInterval(() => {
      void refresh();
    }, 3000);

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-bold mb-4">Process Risk Overview</h2>
      {error && <p className="text-red-600 mb-3">{error}</p>}
      {processes.length === 0 ? (
        <p className="text-gray-600">No process risk data available.</p>
      ) : (
        <div className="overflow-auto">
          <table className="min-w-full text-sm">
            <thead>
              <tr className="text-left border-b">
                <th className="py-2 pr-4">PID</th>
                <th className="py-2 pr-4">Path</th>
                <th className="py-2 pr-4">Risk</th>
                <th className="py-2">Detectors</th>
              </tr>
            </thead>
            <tbody>
              {processes.map((proc) => (
                <tr key={`${proc.processId}-${proc.lastActivity}`} className="border-b last:border-b-0">
                  <td className="py-2 pr-4 font-medium">{proc.processId}</td>
                  <td className="py-2 pr-4 text-gray-700">{proc.processPath || '-'}</td>
                  <td className="py-2 pr-4">{(proc.riskScore * 100).toFixed(1)}%</td>
                  <td className="py-2 text-gray-600">
                    {proc.activeDetectors.length > 0 ? proc.activeDetectors.join(', ') : 'None'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

