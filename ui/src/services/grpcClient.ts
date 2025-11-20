// gRPC Client for SentinelGuard UI
import { SentinelGuardServiceClient } from '../generated/sentinelguard_grpc_pb';
import * as grpc from '@grpc/grpc-js';

const GRPC_SERVER = '127.0.0.1:50051';

export class SentinelGuardClient {
  private client: SentinelGuardServiceClient;

  constructor() {
    this.client = new SentinelGuardServiceClient(
      GRPC_SERVER,
      grpc.credentials.createInsecure()
    );
  }

  async getAlerts(sinceTimestamp: number) {
    // Implementation for GetAlerts RPC
    return new Promise((resolve, reject) => {
      // Placeholder - would use actual gRPC call
      resolve([]);
    });
  }

  async getProcessRiskOverview() {
    // Implementation for GetProcessRiskOverview RPC
    return new Promise((resolve, reject) => {
      // Placeholder
      resolve({ processes: [] });
    });
  }

  async getQuarantinedProcesses() {
    // Implementation for GetQuarantinedProcesses RPC
    return new Promise((resolve, reject) => {
      // Placeholder
      resolve({ processes: [] });
    });
  }

  async getSystemHealth() {
    // Implementation for GetSystemHealth RPC
    return new Promise((resolve, reject) => {
      // Placeholder
      resolve({
        agent_running: true,
        driver_loaded: true,
        events_per_second: 0,
        total_events: 0,
        active_processes: 0,
        quarantined_count: 0,
        cpu_usage: 0.0,
        memory_usage: 0.0,
      });
    });
  }

  async releaseFromQuarantine(processId: number) {
    // Implementation for ReleaseFromQuarantine RPC
    return new Promise((resolve, reject) => {
      // Placeholder
      resolve({ success: true, message: 'Released' });
    });
  }
}

export const grpcClient = new SentinelGuardClient();

