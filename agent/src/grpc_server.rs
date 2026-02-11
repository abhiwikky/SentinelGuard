//
// gRPC Server for UI Communication
//

use anyhow::Result;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{debug, info};
use dashmap::DashMap;

use crate::database::Database;
use crate::quarantine::QuarantineController;

// Include generated protobuf code
pub mod sentinelguard {
    tonic::include_proto!("sentinelguard");
}

use sentinelguard::sentinel_guard_service_server::{SentinelGuardService, SentinelGuardServiceServer};
use sentinelguard::*;

pub struct SentinelGuardServiceImpl {
    db: Arc<Database>,
    quarantine: Arc<QuarantineController>,
    alerts: Arc<DashMap<i64, Alert>>,
    process_risks: Arc<DashMap<u32, ProcessRisk>>,
}

#[tonic::async_trait]
impl SentinelGuardService for SentinelGuardServiceImpl {
    type GetAlertsStream = tokio_stream::wrappers::ReceiverStream<Result<Alert, Status>>;

    async fn get_alerts(
        &self,
        request: Request<GetAlertsRequest>,
    ) -> Result<Response<Self::GetAlertsStream>, Status> {
        let req = request.into_inner();
        let since_timestamp = req.since_timestamp;

        let (tx, rx) = tokio::sync::mpsc::channel(128);

        // Send existing alerts
        for entry in self.alerts.iter() {
            if entry.value().timestamp >= since_timestamp {
                if let Err(_) = tx.send(Ok(entry.value().clone())).await {
                    break;
                }
            }
        }

        // In production, would also set up a stream for new alerts
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn get_process_risk_overview(
        &self,
        _request: Request<GetProcessRiskOverviewRequest>,
    ) -> Result<Response<ProcessRiskOverview>, Status> {
        let mut processes = Vec::new();
        
        for entry in self.process_risks.iter() {
            processes.push(entry.value().clone());
        }

        // Sort by risk score descending
        processes.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());

        Ok(Response::new(ProcessRiskOverview { processes }))
    }

    async fn get_quarantined_processes(
        &self,
        _request: Request<GetQuarantinedProcessesRequest>,
    ) -> Result<Response<QuarantinedProcessesResponse>, Status> {
        // Query database for quarantined processes
        let processes = vec![]; // TODO: Query from database
        
        Ok(Response::new(QuarantinedProcessesResponse { processes }))
    }

    async fn get_detector_logs(
        &self,
        request: Request<GetDetectorLogsRequest>,
    ) -> Result<Response<DetectorLogsResponse>, Status> {
        let req = request.into_inner();
        // TODO: Query detector logs from database
        let entries = vec![];
        
        Ok(Response::new(DetectorLogsResponse { entries }))
    }

    async fn get_system_health(
        &self,
        _request: Request<GetSystemHealthRequest>,
    ) -> Result<Response<SystemHealth>, Status> {
        // TODO: Collect real system metrics
        let health = SystemHealth {
            agent_running: true,
            driver_loaded: true,
            events_per_second: 0,
            total_events: 0,
            active_processes: 0,
            quarantined_count: 0,
            cpu_usage: 0.0,
            memory_usage: 0.0,
        };

        Ok(Response::new(health))
    }

    async fn update_config(
        &self,
        request: Request<UpdateConfigRequest>,
    ) -> Result<Response<UpdateConfigResponse>, Status> {
        let req = request.into_inner();
        debug!("Config update request: {:?}", req.config_updates);
        
        // TODO: Implement config update logic
        Ok(Response::new(UpdateConfigResponse {
            success: true,
            message: "Config updated successfully".to_string(),
        }))
    }

    async fn release_from_quarantine(
        &self,
        request: Request<ReleaseFromQuarantineRequest>,
    ) -> Result<Response<ReleaseFromQuarantineResponse>, Status> {
        let req = request.into_inner();
        let process_id = req.process_id;

        // TODO: Implement release from quarantine
        match self.quarantine.release_process(process_id).await {
            Ok(_) => Ok(Response::new(ReleaseFromQuarantineResponse {
                success: true,
                message: format!("Process {} released from quarantine", process_id),
            })),
            Err(e) => Ok(Response::new(ReleaseFromQuarantineResponse {
                success: false,
                message: format!("Failed to release process: {}", e),
            })),
        }
    }
}

impl SentinelGuardServiceImpl {
    pub fn new(
        db: Arc<Database>,
        quarantine: Arc<QuarantineController>,
    ) -> Self {
        Self {
            db,
            quarantine,
            alerts: Arc::new(DashMap::new()),
            process_risks: Arc::new(DashMap::new()),
        }
    }

    pub fn add_alert(&self, alert: Alert) {
        self.alerts.insert(alert.id, alert);
    }

    pub fn update_process_risk(&self, process_id: u32, risk: ProcessRisk) {
        self.process_risks.insert(process_id, risk);
    }
}

pub async fn start_grpc_server(
    db: Arc<Database>,
    quarantine: Arc<QuarantineController>,
    addr: std::net::SocketAddr,
) -> Result<()> {
    let service = SentinelGuardServiceImpl::new(db, quarantine);
    let server = SentinelGuardServiceServer::new(service);

    info!("Starting gRPC server on {}", addr);

    tonic::transport::Server::builder()
        .add_service(server)
        .serve(addr)
        .await?;

    Ok(())
}

