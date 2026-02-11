import React, { useState, useEffect } from 'react';
import { grpcClient } from '../services/grpcClient';

interface SystemHealthData {
  agentStatus: 'online' | 'offline';
  driverStatus: 'loaded' | 'not_loaded';
  eventsPerSecond: number;
  totalEvents: number;
  activeProcesses: number;
  quarantinedCount: number;
  cpuUsage: number;
  memoryUsage: number;
}

export const SystemHealth: React.FC = () => {
  const [health, setHealth] = useState<SystemHealthData>({
    agentStatus: 'offline',
    driverStatus: 'not_loaded',
    eventsPerSecond: 0,
    totalEvents: 0,
    activeProcesses: 0,
    quarantinedCount: 0,
    cpuUsage: 0,
    memoryUsage: 0,
  });
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    const refresh = async () => {
      try {
        const status = await grpcClient.getSystemHealth();
        if (!mounted) {
          return;
        }
        setError(null);
        setHealth({
          agentStatus: status.agentRunning ? 'online' : 'offline',
          driverStatus: status.driverLoaded ? 'loaded' : 'not_loaded',
          eventsPerSecond: status.eventsPerSecond,
          totalEvents: status.totalEvents,
          activeProcesses: status.activeProcesses,
          quarantinedCount: status.quarantinedCount,
          cpuUsage: status.cpuUsage,
          memoryUsage: status.memoryUsage,
        });
      } catch (err) {
        if (!mounted) {
          return;
        }
        setError(err instanceof Error ? err.message : 'Unable to fetch system health');
      }
    };

    void refresh();
    const interval = setInterval(() => {
      void refresh();
    }, 2000);

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 col-span-full text-red-700">
          {error}
        </div>
      )}
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
        <h3 className="text-lg font-semibold mb-2">Events/sec</h3>
        <p className="text-2xl text-orange-600">{health.eventsPerSecond}</p>
      </div>

      <div className="bg-white p-6 rounded-lg shadow">
        <h3 className="text-lg font-semibold mb-2">Active Processes</h3>
        <p className="text-2xl text-indigo-600">{health.activeProcesses}</p>
      </div>

      <div className="bg-white p-6 rounded-lg shadow col-span-full">
        <h3 className="text-lg font-semibold mb-4">Quarantined Processes</h3>
        <p className="text-3xl text-red-600">{health.quarantinedCount}</p>
        <p className="text-sm text-gray-500 mt-2">
          CPU: {health.cpuUsage.toFixed(1)}% | Memory: {health.memoryUsage.toFixed(1)}%
        </p>
      </div>
    </div>
  );
};

