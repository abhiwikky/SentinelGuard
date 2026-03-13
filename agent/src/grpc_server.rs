//
// gRPC Server for UI Communication
//

use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc, Mutex};
use tonic::{Request, Response, Status};
use tracing::{debug, info};

use crate::database::Database;
use crate::quarantine::QuarantineController;

// Include generated protobuf code
pub mod sentinelguard {
    tonic::include_proto!("sentinelguard");
}

use sentinelguard::sentinel_guard_service_server::{
    SentinelGuardService, SentinelGuardServiceServer,
};
use sentinelguard::*;

#[cfg(windows)]
use windows::Win32::Foundation::FILETIME;
#[cfg(windows)]
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
#[cfg(windows)]
use windows::Win32::System::SystemInformation::{GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO};
#[cfg(windows)]
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

#[derive(Debug)]
struct CpuSampler {
    last_wall: Instant,
    last_process_cpu_100ns: u64,
}

impl CpuSampler {
    fn new() -> Self {
        Self {
            last_wall: Instant::now(),
            last_process_cpu_100ns: 0,
        }
    }
}

pub struct DashboardState {
    alerts: DashMap<i64, Alert>,
    process_risks: DashMap<u32, ProcessRisk>,
    alert_tx: broadcast::Sender<Alert>,
    last_alert_timestamp_by_process: DashMap<u32, i64>,
    cpu_sampler: Mutex<CpuSampler>,
}

impl DashboardState {
    pub fn new() -> Arc<Self> {
        let (alert_tx, _) = broadcast::channel(256);
        Arc::new(Self {
            alerts: DashMap::new(),
            process_risks: DashMap::new(),
            alert_tx,
            last_alert_timestamp_by_process: DashMap::new(),
            cpu_sampler: Mutex::new(CpuSampler::new()),
        })
    }

    pub fn add_alert(&self, alert: Alert) {
        self.last_alert_timestamp_by_process
            .insert(alert.process_id, alert.timestamp);
        self.alerts.insert(alert.id, alert.clone());
        let _ = self.alert_tx.send(alert);
    }

    pub fn should_emit_alert(&self, process_id: u32, timestamp: i64) -> bool {
        match self.last_alert_timestamp_by_process.get(&process_id) {
            Some(last) => timestamp.saturating_sub(*last) >= 15,
            None => true,
        }
    }

    pub fn update_process_risk(&self, risk: ProcessRisk) {
        self.process_risks.insert(risk.process_id, risk);
    }

    pub fn list_process_risks(&self) -> Vec<ProcessRisk> {
        let mut processes: Vec<_> = self.process_risks.iter().map(|entry| entry.value().clone()).collect();
        processes.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());
        processes
    }

    pub fn list_alerts_since(&self, since_timestamp: i64) -> Vec<Alert> {
        let mut alerts: Vec<_> = self
            .alerts
            .iter()
            .filter(|entry| entry.value().timestamp >= since_timestamp)
            .map(|entry| entry.value().clone())
            .collect();
        alerts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        alerts
    }

    pub fn list_quarantined_processes(&self) -> Vec<QuarantinedProcess> {
        self.list_alerts_since(0)
            .into_iter()
            .filter(|alert| alert.quarantined)
            .map(|alert| QuarantinedProcess {
                process_id: alert.process_id,
                process_path: alert.process_path,
                ml_score: alert.ml_score,
                quarantined_at: alert.timestamp,
                reason: if alert.triggered_detectors.is_empty() {
                    "ml_threshold".to_string()
                } else {
                    alert.triggered_detectors.join(", ")
                },
            })
            .collect()
    }

    pub fn list_detector_logs(&self, since_timestamp: i64, limit: usize) -> Vec<DetectorLogEntry> {
        let mut entries = Vec::new();

        for risk in self.list_process_risks() {
            if risk.last_activity < since_timestamp {
                continue;
            }

            for detector_name in &risk.active_detectors {
                entries.push(DetectorLogEntry {
                    detector_name: detector_name.clone(),
                    process_id: risk.process_id,
                    score: risk.risk_score,
                    timestamp: risk.last_activity,
                    details: format!("{} flagged {}", detector_name, risk.process_path),
                });
            }
        }

        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        entries.truncate(limit);
        entries
    }

    pub async fn sample_resource_usage(&self) -> (f64, f64) {
        #[cfg(windows)]
        {
            let cpu = sample_cpu_usage(&self.cpu_sampler).await.unwrap_or(0.0);
            let memory = sample_memory_usage().unwrap_or(0.0);
            return (cpu, memory);
        }

        #[allow(unreachable_code)]
        (0.0, 0.0)
    }
}

pub struct SentinelGuardServiceImpl {
    db: Arc<Database>,
    quarantine: Arc<QuarantineController>,
    state: Arc<DashboardState>,
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
        let (tx, rx) = mpsc::channel(128);

        for alert in self.state.list_alerts_since(since_timestamp) {
            if tx.send(Ok(alert)).await.is_err() {
                return Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)));
            }
        }

        let mut alert_rx = self.state.alert_tx.subscribe();
        tokio::spawn(async move {
            loop {
                match alert_rx.recv().await {
                    Ok(alert) if alert.timestamp >= since_timestamp => {
                        if tx.send(Ok(alert)).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn get_process_risk_overview(
        &self,
        _request: Request<GetProcessRiskOverviewRequest>,
    ) -> Result<Response<ProcessRiskOverview>, Status> {
        Ok(Response::new(ProcessRiskOverview {
            processes: self.state.list_process_risks(),
        }))
    }

    async fn get_quarantined_processes(
        &self,
        _request: Request<GetQuarantinedProcessesRequest>,
    ) -> Result<Response<QuarantinedProcessesResponse>, Status> {
        Ok(Response::new(QuarantinedProcessesResponse {
            processes: self.state.list_quarantined_processes(),
        }))
    }

    async fn get_detector_logs(
        &self,
        request: Request<GetDetectorLogsRequest>,
    ) -> Result<Response<DetectorLogsResponse>, Status> {
        let req = request.into_inner();
        Ok(Response::new(DetectorLogsResponse {
            entries: self
                .state
                .list_detector_logs(req.since_timestamp, req.limit.max(1) as usize),
        }))
    }

    async fn get_system_health(
        &self,
        _request: Request<GetSystemHealthRequest>,
    ) -> Result<Response<SystemHealth>, Status> {
        let (total_events, events_last_5s, active_processes, quarantined_count) = self
            .db
            .get_system_metrics()
            .await
            .map_err(|e| Status::internal(format!("Failed to query system metrics: {}", e)))?;

        let eps = if events_last_5s <= 0 {
            0
        } else {
            ((events_last_5s as f64) / 5.0).round() as i64
        };

        let (cpu_usage, memory_usage) = self.state.sample_resource_usage().await;

        Ok(Response::new(SystemHealth {
            agent_running: true,
            driver_loaded: false,
            events_per_second: eps,
            total_events,
            active_processes,
            quarantined_count,
            cpu_usage,
            memory_usage,
        }))
    }

    async fn update_config(
        &self,
        request: Request<UpdateConfigRequest>,
    ) -> Result<Response<UpdateConfigResponse>, Status> {
        let req = request.into_inner();
        debug!("Config update request: {:?}", req.config_updates);

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
        state: Arc<DashboardState>,
    ) -> Self {
        Self {
            db,
            quarantine,
            state,
        }
    }
}

pub async fn start_grpc_server(
    db: Arc<Database>,
    quarantine: Arc<QuarantineController>,
    state: Arc<DashboardState>,
    addr: std::net::SocketAddr,
) -> Result<()> {
    let service = SentinelGuardServiceImpl::new(db, quarantine, state);
    let server = SentinelGuardServiceServer::new(service);

    info!("Starting gRPC server on {}", addr);

    tonic::transport::Server::builder()
        .add_service(server)
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(windows)]
async fn sample_cpu_usage(cpu_sampler: &Mutex<CpuSampler>) -> Option<f64> {
    let process_handle = unsafe { GetCurrentProcess() };
    let process_cpu_100ns = unsafe { current_process_cpu_100ns(process_handle)? };

    let mut sampler = cpu_sampler.lock().await;
    let now = Instant::now();
    let wall_elapsed = now.duration_since(sampler.last_wall).as_secs_f64();

    if sampler.last_process_cpu_100ns == 0 || wall_elapsed <= f64::EPSILON {
        sampler.last_wall = now;
        sampler.last_process_cpu_100ns = process_cpu_100ns;
        return Some(0.0);
    }

    let cpu_delta_100ns = process_cpu_100ns.saturating_sub(sampler.last_process_cpu_100ns);
    let cpu_delta_seconds = (cpu_delta_100ns as f64) / 10_000_000.0;
    let processor_count = unsafe {
        let mut system_info = SYSTEM_INFO::default();
        GetSystemInfo(&mut system_info);
        system_info.dwNumberOfProcessors.max(1) as f64
    };

    sampler.last_wall = now;
    sampler.last_process_cpu_100ns = process_cpu_100ns;

    Some(((cpu_delta_seconds / (wall_elapsed * processor_count)) * 100.0).clamp(0.0, 100.0))
}

#[cfg(windows)]
unsafe fn current_process_cpu_100ns(process_handle: windows::Win32::Foundation::HANDLE) -> Option<u64> {
    let mut creation_time = FILETIME::default();
    let mut exit_time = FILETIME::default();
    let mut kernel_time = FILETIME::default();
    let mut user_time = FILETIME::default();

    if GetProcessTimes(
        process_handle,
        &mut creation_time,
        &mut exit_time,
        &mut kernel_time,
        &mut user_time,
    )
    .is_err()
    {
        return None;
    }

    Some(filetime_to_u64(kernel_time) + filetime_to_u64(user_time))
}

#[cfg(windows)]
fn filetime_to_u64(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | (value.dwLowDateTime as u64)
}

#[cfg(windows)]
fn sample_memory_usage() -> Option<f64> {
    unsafe {
        let process_handle = GetCurrentProcess();
        let mut counters = PROCESS_MEMORY_COUNTERS::default();
        if !K32GetProcessMemoryInfo(
            process_handle,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
        .as_bool()
        {
            return None;
        }

        let mut memory_status = MEMORYSTATUSEX::default();
        memory_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut memory_status).is_err() || memory_status.ullTotalPhys == 0 {
            return None;
        }

        Some(((counters.WorkingSetSize as f64) / (memory_status.ullTotalPhys as f64) * 100.0).clamp(0.0, 100.0))
    }
}
