import React from 'react';
import {
  useComponentHealthData,
  useDashboardData,
  useSystemHealthData,
} from '../context/DashboardDataContext';

function statusClass(ok: boolean) {
  return ok ? 'text-green-600' : 'text-red-600';
}

export const SystemHealth: React.FC = () => {
  const componentHealth = useComponentHealthData();
  const systemHealth = useSystemHealthData();
  const { refreshError, snapshot } = useDashboardData();
  const systemError = snapshot?.errors.systemHealth || refreshError;

  return (
    <div className="space-y-6">
      {systemError && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">
          {systemError}
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">UI Runtime</h3>
          <p className="text-2xl text-blue-600">Browser</p>
          <p className="text-sm text-gray-500 mt-2">Separate browser tab or window</p>
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">Bridge Process</h3>
          <p className={`text-2xl ${statusClass(componentHealth.webBridgeRunning)}`}>
            {componentHealth.webBridgeRunning ? 'Healthy' : 'Unavailable'}
          </p>
          <p className="text-sm text-gray-500 mt-2">{componentHealth.grpcAddress}</p>
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">gRPC Backend</h3>
          <p className={`text-2xl ${statusClass(componentHealth.grpcReachable)}`}>
            {componentHealth.grpcReachable ? 'Reachable' : 'Down'}
          </p>
          {componentHealth.lastError && (
            <p className="text-sm text-red-600 mt-2">{componentHealth.lastError}</p>
          )}
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">Agent Status</h3>
          <p className={`text-2xl ${statusClass(Boolean(systemHealth?.agentRunning))}`}>
            {systemHealth?.agentRunning ? 'Online' : 'Offline'}
          </p>
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">Driver Status</h3>
          <p className={`text-2xl ${statusClass(Boolean(systemHealth?.driverLoaded))}`}>
            {systemHealth?.driverLoaded ? 'Loaded' : 'Not Loaded'}
          </p>
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">Total Events</h3>
          <p className="text-2xl text-blue-600">
            {(systemHealth?.totalEvents || 0).toLocaleString()}
          </p>
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">Events/sec</h3>
          <p className="text-2xl text-orange-600">{systemHealth?.eventsPerSecond || 0}</p>
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">Active Processes</h3>
          <p className="text-2xl text-indigo-600">{systemHealth?.activeProcesses || 0}</p>
        </div>

        <div className="bg-white p-6 rounded-lg shadow col-span-full">
          <h3 className="text-lg font-semibold mb-4">Quarantine and Resource Summary</h3>
          <p className="text-3xl text-red-600">{systemHealth?.quarantinedCount || 0}</p>
          <p className="text-sm text-gray-500 mt-2">
            CPU: {(systemHealth?.cpuUsage || 0).toFixed(1)}% | Memory:{' '}
            {(systemHealth?.memoryUsage || 0).toFixed(1)}%
          </p>
        </div>
      </div>
    </div>
  );
};
