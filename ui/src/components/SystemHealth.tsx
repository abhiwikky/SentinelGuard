import React, { useState, useEffect } from 'react';

interface SystemHealthData {
  agentStatus: 'online' | 'offline';
  driverStatus: 'loaded' | 'not_loaded';
  totalEvents: number;
  activeDetections: number;
  quarantinedCount: number;
}

export const SystemHealth: React.FC = () => {
  const [health, setHealth] = useState<SystemHealthData>({
    agentStatus: 'offline',
    driverStatus: 'not_loaded',
    totalEvents: 0,
    activeDetections: 0,
    quarantinedCount: 0,
  });

  useEffect(() => {
    // TODO: Connect to gRPC API
    const interval = setInterval(() => {
      // Simulated data
      setHealth({
        agentStatus: 'online',
        driverStatus: 'loaded',
        totalEvents: Math.floor(Math.random() * 10000),
        activeDetections: Math.floor(Math.random() * 10),
        quarantinedCount: Math.floor(Math.random() * 5),
      });
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      <div className="bg-white p-6 rounded-lg shadow">
        <h3 className="text-lg font-semibold mb-2">Agent Status</h3>
        <p className={`text-2xl ${health.agentStatus === 'online' ? 'text-green-600' : 'text-red-600'}`}>
          {health.agentStatus === 'online' ? 'Online' : 'Offline'}
        </p>
      </div>

      <div className="bg-white p-6 rounded-lg shadow">
        <h3 className="text-lg font-semibold mb-2">Driver Status</h3>
        <p className={`text-2xl ${health.driverStatus === 'loaded' ? 'text-green-600' : 'text-red-600'}`}>
          {health.driverStatus === 'loaded' ? 'Loaded' : 'Not Loaded'}
        </p>
      </div>

      <div className="bg-white p-6 rounded-lg shadow">
        <h3 className="text-lg font-semibold mb-2">Total Events</h3>
        <p className="text-2xl text-blue-600">{health.totalEvents.toLocaleString()}</p>
      </div>

      <div className="bg-white p-6 rounded-lg shadow">
        <h3 className="text-lg font-semibold mb-2">Active Detections</h3>
        <p className="text-2xl text-orange-600">{health.activeDetections}</p>
      </div>

      <div className="bg-white p-6 rounded-lg shadow col-span-full">
        <h3 className="text-lg font-semibold mb-4">Quarantined Processes</h3>
        <p className="text-3xl text-red-600">{health.quarantinedCount}</p>
      </div>
    </div>
  );
};

