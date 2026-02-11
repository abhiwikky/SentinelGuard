import React, { useState, useEffect } from 'react';
import { grpcClient, type Alert as RpcAlert } from '../services/grpcClient';

interface Alert {
  id: string;
  timestamp: Date;
  processId: number;
  processPath: string;
  mlScore: number;
  quarantined: boolean;
}

function normalizeTimestamp(value: number): Date {
  if (!Number.isFinite(value)) {
    return new Date();
  }
  const millis = value > 1_000_000_000_000 ? value : value * 1000;
  return new Date(millis);
}

function mapAlert(payload: RpcAlert): Alert {
  return {
    id: String(payload.id),
    timestamp: normalizeTimestamp(payload.timestamp),
    processId: payload.processId,
    processPath: payload.processPath,
    mlScore: payload.mlScore,
    quarantined: payload.quarantined,
  };
}

export const AlertFeed: React.FC = () => {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [streamError, setStreamError] = useState<string | null>(null);

  useEffect(() => {
    let unsubscribe = () => {};
    let mounted = true;

    const start = async () => {
      try {
        unsubscribe = grpcClient.onAlert((payload) => {
          if (!mounted) {
            return;
          }
          if ('__streamError' in payload) {
            setStreamError(payload.message);
            return;
          }
          setStreamError(null);
          setAlerts((prev) => [mapAlert(payload), ...prev].slice(0, 50));
        });
        await grpcClient.startAlertStream(0);
      } catch (error) {
        if (!mounted) {
          return;
        }
        const message = error instanceof Error ? error.message : 'Unable to start alert stream';
        setStreamError(message);
      }
    };

    start();

    return () => {
      mounted = false;
      unsubscribe();
      void grpcClient.stopAlertStream();
    };
  }, []);

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-bold mb-4">Live Alerts</h2>
      {streamError && (
        <div className="mb-4 rounded border border-red-200 bg-red-50 p-3 text-sm text-red-700">
          Stream error: {streamError}
        </div>
      )}
      <div className="space-y-4">
        {alerts.map(alert => (
          <div
            key={alert.id}
            className={`p-4 rounded border-l-4 ${
              alert.quarantined ? 'border-green-500 bg-green-50' : 'border-red-500 bg-red-50'
            }`}
          >
            <div className="flex justify-between items-start">
              <div>
                <p className="font-semibold">Process ID: {alert.processId}</p>
                <p className="text-sm text-gray-600">{alert.processPath}</p>
                <p className="text-xs text-gray-500 mt-1">
                  {alert.timestamp.toLocaleString()}
                </p>
              </div>
              <div className="text-right">
                <p className="text-lg font-bold">ML Score: {(alert.mlScore * 100).toFixed(1)}%</p>
                <span
                  className={`px-2 py-1 rounded text-xs ${
                    alert.quarantined
                      ? 'bg-green-200 text-green-800'
                      : 'bg-red-200 text-red-800'
                  }`}
                >
                  {alert.quarantined ? 'Quarantined' : 'Active'}
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

