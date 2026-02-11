const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('sentinelguardApi', {
  getSystemHealth: () => ipcRenderer.invoke('sg:get-system-health'),
  getProcessRiskOverview: (limit) => ipcRenderer.invoke('sg:get-process-risk-overview', limit),
  getQuarantinedProcesses: () => ipcRenderer.invoke('sg:get-quarantined-processes'),
  getDetectorLogs: (sinceTimestamp, limit) =>
    ipcRenderer.invoke('sg:get-detector-logs', sinceTimestamp, limit),
  releaseFromQuarantine: (processId) => ipcRenderer.invoke('sg:release-from-quarantine', processId),
  startAlertStream: (sinceTimestamp) => ipcRenderer.invoke('sg:alerts:start', sinceTimestamp),
  stopAlertStream: () => ipcRenderer.invoke('sg:alerts:stop'),
  onAlert: (callback) => {
    const listener = (_event, payload) => callback(payload);
    ipcRenderer.on('sg:alerts:new', listener);
    return () => ipcRenderer.removeListener('sg:alerts:new', listener);
  },
});
