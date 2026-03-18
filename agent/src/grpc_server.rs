//! SentinelGuard gRPC Server Module
//!
//! Implements the SentinelGuardService gRPC API using Tonic.

use crate::correlation::Correlator;
use crate::database::Database;
use crate::events;
use crate::quarantine::QuarantineManager;
use crate::telemetry::Metrics;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Request, Response, Status};
use tracing::{error, info};

/// Include generated protobuf code
pub mod proto {
    tonic::include_proto!("sentinelguard");
}

use proto::sentinel_guard_service_server::{SentinelGuardService, SentinelGuardServiceServer};

/// Shared state accessible to the gRPC handlers
pub struct ServiceState {
    pub database: Arc<Database>,
    pub correlator: Arc<Correlator>,
    pub quarantine: Arc<QuarantineManager>,
    pub metrics: Arc<Metrics>,
    pub driver_connected: Arc<std::sync::atomic::AtomicBool>,
    pub model_loaded: Arc<std::sync::atomic::AtomicBool>,
    pub alert_broadcaster: broadcast::Sender<proto::Alert>,
    pub agent_version: String,
}

pub struct SentinelGuardServiceImpl {
    state: Arc<ServiceState>,
}

impl SentinelGuardServiceImpl {
    pub fn new(state: Arc<ServiceState>) -> Self {
        Self { state }
    }

    pub fn into_server(self) -> SentinelGuardServiceServer<Self> {
        SentinelGuardServiceServer::new(self)
    }
}

/// Convert internal Alert to protobuf Alert
fn alert_to_proto(alert: &events::Alert) -> proto::Alert {
    proto::Alert {
        alert_id: alert.alert_id,
        process_id: alert.process_id,
        process_name: alert.process_name.clone(),
        severity: match alert.severity {
            events::Severity::Low => proto::Severity::Low as i32,
            events::Severity::Medium => proto::Severity::Medium as i32,
            events::Severity::High => proto::Severity::High as i32,
            events::Severity::Critical => proto::Severity::Critical as i32,
            _ => proto::Severity::Unknown as i32,
        },
        risk_score: alert.risk_score,
        description: alert.description.clone(),
        detector_results: alert
            .detector_results
            .iter()
            .map(detector_result_to_proto)
            .collect(),
        quarantine_status: match alert.quarantine_status {
            events::QuarantineStatus::Suspended => proto::QuarantineStatus::QsSuspended as i32,
            events::QuarantineStatus::Released => proto::QuarantineStatus::QsReleased as i32,
            events::QuarantineStatus::ProcessExited => {
                proto::QuarantineStatus::QsProcessExited as i32
            }
            _ => proto::QuarantineStatus::QsUnknown as i32,
        },
        timestamp_ns: alert.timestamp_ns,
    }
}

fn detector_result_to_proto(result: &events::DetectorResult) -> proto::DetectorResult {
    proto::DetectorResult {
        detector_name: result.detector_name.clone(),
        score: result.score,
        evidence: result.evidence.clone(),
        timestamp_ns: result.timestamp_ns,
        process_id: result.process_id,
    }
}

#[tonic::async_trait]
impl SentinelGuardService for SentinelGuardServiceImpl {
    async fn get_health(
        &self,
        _request: Request<proto::GetHealthRequest>,
    ) -> Result<Response<proto::GetHealthResponse>, Status> {
        let db = self.state.database.clone();
        let db_connected = tokio::task::spawn_blocking(move || db.is_connected())
            .await
            .unwrap_or(false);

        let driver_connected = self
            .state
            .driver_connected
            .load(std::sync::atomic::Ordering::Relaxed);
        let model_loaded = self
            .state
            .model_loaded
            .load(std::sync::atomic::Ordering::Relaxed);

        let health = proto::HealthStatus {
            agent_running: true,
            driver_connected,
            model_loaded,
            database_connected: db_connected,
            events_processed: self.state.metrics.events_processed(),
            alerts_generated: self.state.metrics.alerts_generated(),
            uptime_seconds: self.state.metrics.uptime_seconds(),
            events_per_second: self.state.metrics.events_per_second(),
            agent_version: self.state.agent_version.clone(),
        };

        Ok(Response::new(proto::GetHealthResponse {
            health: Some(health),
        }))
    }

    async fn get_alerts(
        &self,
        request: Request<proto::GetAlertsRequest>,
    ) -> Result<Response<proto::GetAlertsResponse>, Status> {
        let req = request.into_inner();

        let db = self.state.database.clone();
        let alerts = tokio::task::spawn_blocking(move || db.get_alerts(req.limit, req.since_ns))
            .await
            .map_err(|e| Status::internal("Task join error"))?
            .map_err(|e| {
                error!("Failed to get alerts: {}", e);
                Status::internal("Database error")
            })?;

        let proto_alerts: Vec<proto::Alert> = alerts.iter().map(alert_to_proto).collect();

        Ok(Response::new(proto::GetAlertsResponse {
            alerts: proto_alerts,
        }))
    }

    type StreamAlertsStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<proto::Alert, Status>> + Send>>;

    async fn stream_alerts(
        &self,
        _request: Request<proto::StreamAlertsRequest>,
    ) -> Result<Response<Self::StreamAlertsStream>, Status> {
        let rx = self.state.alert_broadcaster.subscribe();
        let stream = BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(alert) => Some(Ok(alert)),
            Err(_) => None, // Skip lagged messages
        });

        Ok(Response::new(Box::pin(stream)))
    }

    async fn get_process_risk(
        &self,
        request: Request<proto::GetProcessRiskRequest>,
    ) -> Result<Response<proto::GetProcessRiskResponse>, Status> {
        let _req = request.into_inner();

        let scores = self.state.correlator.get_all_scores();

        // Get quarantined PIDs
        let db = self.state.database.clone();
        let quarantined = tokio::task::spawn_blocking(move || db.get_quarantined_processes())
            .await
            .unwrap_or_else(|_| Ok(Vec::new()))
            .unwrap_or_default();
        
        let quarantined_pids: std::collections::HashSet<u32> =
            quarantined.iter().map(|q| q.0).collect();

        let processes: Vec<proto::ProcessRiskEntry> = scores
            .iter()
            .map(|s| proto::ProcessRiskEntry {
                process_id: s.process_id,
                process_name: s.process_name.clone(),
                current_risk_score: s.final_score,
                event_count: s.detector_results.len() as u64,
                last_event_ns: s.window_end_ns,
                is_quarantined: quarantined_pids.contains(&s.process_id),
            })
            .collect();

        Ok(Response::new(proto::GetProcessRiskResponse { processes }))
    }

    async fn get_quarantined(
        &self,
        _request: Request<proto::GetQuarantinedRequest>,
    ) -> Result<Response<proto::GetQuarantinedResponse>, Status> {
        let db = self.state.database.clone();
        let quarantined = tokio::task::spawn_blocking(move || db.get_quarantined_processes())
            .await
            .map_err(|_| Status::internal("Task join error"))?
            .map_err(|e| {
                error!("Failed to get quarantined processes: {}", e);
                Status::internal("Database error")
            })?;

        let processes: Vec<proto::QuarantinedProcess> = quarantined
            .iter()
            .map(|(pid, name, score, ts, _status)| proto::QuarantinedProcess {
                process_id: *pid,
                process_name: name.clone(),
                risk_score: *score,
                quarantined_at_ns: *ts,
                status: proto::QuarantineStatus::QsSuspended as i32,
            })
            .collect();

        Ok(Response::new(proto::GetQuarantinedResponse { processes }))
    }

    async fn release_process(
        &self,
        request: Request<proto::ReleaseProcessRequest>,
    ) -> Result<Response<proto::ReleaseProcessResponse>, Status> {
        let pid = request.into_inner().process_id;

        let result = self
            .state
            .quarantine
            .release_process(pid)
            .map_err(|e| {
                error!("Failed to release process {}: {}", pid, e);
                Status::internal("Quarantine helper error")
            })?;

        if result.success {
            // Log the release
            let db = self.state.database.clone();
            tokio::task::spawn_blocking(move || {
                db.log_quarantine(
                    pid,
                    "",
                    0.0,
                    "release",
                    events::QuarantineStatus::Released,
                )
                .ok();
            }).await.ok();
        }

        Ok(Response::new(proto::ReleaseProcessResponse {
            success: result.success,
            message: result.message,
        }))
    }

    async fn get_detector_logs(
        &self,
        request: Request<proto::GetDetectorLogsRequest>,
    ) -> Result<Response<proto::GetDetectorLogsResponse>, Status> {
        let req = request.into_inner();

        let db = self.state.database.clone();
        let results = tokio::task::spawn_blocking(move || db.get_detector_results(req.limit, req.since_ns))
            .await
            .map_err(|_| Status::internal("Task join error"))?
            .map_err(|e| {
                error!("Failed to get detector logs: {}", e);
                Status::internal("Database error")
            })?;

        let proto_results: Vec<proto::DetectorResult> =
            results.iter().map(detector_result_to_proto).collect();

        Ok(Response::new(proto::GetDetectorLogsResponse {
            results: proto_results,
        }))
    }
}

/// Create and start the gRPC server
pub async fn start_grpc_server(
    listen_addr: &str,
    state: Arc<ServiceState>,
) -> anyhow::Result<()> {
    let addr = listen_addr.parse().map_err(|e| {
        anyhow::anyhow!("Invalid gRPC listen address '{}': {}", listen_addr, e)
    })?;

    info!("Starting gRPC server on {}", listen_addr);

    let service = SentinelGuardServiceImpl::new(state);

    tonic::transport::Server::builder()
        .add_service(service.into_server())
        .serve(addr)
        .await
        .map_err(|e| anyhow::anyhow!("gRPC server error: {}", e))?;

    Ok(())
}

/// Use filter_map for broadcast stream
use tokio_stream::StreamExt;
