import React, { useState, useEffect } from 'react';

interface Alert {
  id: string;
  timestamp: Date;
  processId: number;
  processPath: string;
  mlScore: number;
  quarantined: boolean;
}

export const AlertFeed: React.FC = () => {
  const [alerts, setAlerts] = useState<Alert[]>([]);

  useEffect(() => {
    // TODO: Connect to gRPC API for real-time alerts
    const interval = setInterval(() => {
      // Simulated alerts
      setAlerts(prev => [
        {
          id: Math.random().toString(),
          timestamp: new Date(),
          processId: Math.floor(Math.random() * 10000),
          processPath: 'C:\\Users\\Test\\malware.exe',
          mlScore: 0.85,
          quarantined: true,
        },
        ...prev.slice(0, 49),
      ]);
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-bold mb-4">Live Alerts</h2>
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

