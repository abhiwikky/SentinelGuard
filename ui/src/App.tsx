import { useState } from 'react';
import { AlertFeed } from './components/AlertFeed';
import { ProcessRiskOverview } from './components/ProcessRiskOverview';
import { QuarantinedProcesses } from './components/QuarantinedProcesses';
import { SystemHealth } from './components/SystemHealth';
import { DetectorLogs } from './components/DetectorLogs';
import './App.css';

function App() {
  const [activeTab, setActiveTab] = useState('dashboard');

  return (
    <div className="min-h-screen bg-gray-100">
      <nav className="bg-blue-600 text-white p-4">
        <div className="container mx-auto flex justify-between items-center">
          <h1 className="text-2xl font-bold">SentinelGuard</h1>
          <div className="flex space-x-4">
            <button
              onClick={() => setActiveTab('dashboard')}
              className={`px-4 py-2 rounded ${activeTab === 'dashboard' ? 'bg-blue-700' : ''}`}
            >
              Dashboard
            </button>
            <button
              onClick={() => setActiveTab('alerts')}
              className={`px-4 py-2 rounded ${activeTab === 'alerts' ? 'bg-blue-700' : ''}`}
            >
              Alerts
            </button>
            <button
              onClick={() => setActiveTab('processes')}
              className={`px-4 py-2 rounded ${activeTab === 'processes' ? 'bg-blue-700' : ''}`}
            >
              Processes
            </button>
            <button
              onClick={() => setActiveTab('quarantine')}
              className={`px-4 py-2 rounded ${activeTab === 'quarantine' ? 'bg-blue-700' : ''}`}
            >
              Quarantine
            </button>
            <button
              onClick={() => setActiveTab('logs')}
              className={`px-4 py-2 rounded ${activeTab === 'logs' ? 'bg-blue-700' : ''}`}
            >
              Logs
            </button>
          </div>
        </div>
      </nav>

      <main className="container mx-auto p-6">
        {activeTab === 'dashboard' && <SystemHealth />}
        {activeTab === 'alerts' && <AlertFeed />}
        {activeTab === 'processes' && <ProcessRiskOverview />}
        {activeTab === 'quarantine' && <QuarantinedProcesses />}
        {activeTab === 'logs' && <DetectorLogs />}
      </main>
    </div>
  );
}

export default App;

