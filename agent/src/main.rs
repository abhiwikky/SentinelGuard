//
// SentinelGuard User-Mode Agent
// Main entry point
//

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tracing::{error, info};

mod communication;
mod config;
mod correlation;
mod database;
mod detectors;
mod events;
mod grpc_server;
mod quarantine;
mod security;
mod service;
mod telemetry;

use communication::KernelCommunication;
use config::Config;
use correlation::CorrelationEngine;
use database::Database;
use detectors::DetectorManager;
use events::EventIngestion;
use grpc_server::{start_grpc_server, DashboardState};
use quarantine::QuarantineController;
use security::SecurityModule;

pub(crate) enum ShutdownMode {
    Console,
    Service(watch::Receiver<bool>),
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("sentinelguard_agent=info")
        .init();

    if service::try_run_service()? {
        return Ok(());
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(run_agent(ShutdownMode::Console))
}

pub(crate) async fn run_agent(shutdown_mode: ShutdownMode) -> Result<()> {
    info!("SentinelGuard Agent starting...");

    // Load configuration
    let config = Arc::new(Config::load()?);
    info!("Configuration loaded");

    // Initialize security module
    let security = Arc::new(SecurityModule::new()?);
    security.protect_process()?;
    security.enable_tamper_detection()?;
    info!("Security module initialized");

    // Initialize database
    let db = Arc::new(Database::new(&config.database_path).await?);
    db.initialize_schema().await?;
    info!("Database initialized");

    // Initialize quarantine controller
    let quarantine = Arc::new(QuarantineController::new(&config.quarantine_path)?);
    info!("Quarantine controller initialized");

    // Shared dashboard state for gRPC/UI
    let dashboard_state = DashboardState::new();

    // Initialize ML correlation engine
    let correlation_engine = Arc::new(CorrelationEngine::new(&config.ml_model_path).await?);
    info!("ML correlation engine initialized");

    // Initialize detector manager
    let detector_manager = Arc::new(DetectorManager::new(config.clone()).await?);
    info!("Detector manager initialized");

    // Create event channels
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (detector_tx, detector_rx) = mpsc::unbounded_channel();

    // Start event ingestion from kernel
    let kernel_comm = KernelCommunication::new(event_tx.clone(), config.communication.clone())?;
    tokio::spawn(async move {
        if let Err(e) = kernel_comm.start().await {
            error!("Kernel communication error: {}", e);
        }
    });

    // Start event ingestion processor
    let ingestion = EventIngestion::new(event_rx, detector_tx.clone(), db.clone());
    tokio::spawn(async move {
        if let Err(e) = ingestion.start().await {
            error!("Event ingestion error: {}", e);
        }
    });

    // Start detector manager
    let detector_manager_clone = detector_manager.clone();
    tokio::spawn(async move {
        if let Err(e) = detector_manager_clone.process_events(detector_rx).await {
            error!("Detector manager error: {}", e);
        }
    });

    // Start correlation and quarantine loop
    let correlation_engine_final = correlation_engine.clone();
    let quarantine_final = quarantine.clone();
    let db_final = db.clone();
    let config_final = config.clone();
    let dashboard_state_final = dashboard_state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let risk_snapshots = detector_manager.get_process_risk_snapshots().await;
            for risk in &risk_snapshots {
                dashboard_state_final.update_process_risk(
                    grpc_server::sentinelguard::ProcessRisk {
                        process_id: risk.process_id,
                        process_path: risk.process_path.clone(),
                        risk_score: risk.risk_score as f64,
                        last_activity: risk.last_activity,
                        active_detectors: risk.active_detectors.clone(),
                    }
                );
            }

            // Get aggregated scores from detectors
            let scores = detector_manager.get_aggregated_scores().await;

            // Run ML correlation
            if scores.process_id != 0 {
                if let Err(e) = db_final.store_detector_scores(&scores).await {
                    error!("Failed to store detector scores: {}", e);
                }

                if let Ok(ml_score) = correlation_engine_final.infer(&scores).await {
                    if let Err(e) = db_final
                        .log_ml_result(scores.process_id, ml_score, scores.timestamp)
                        .await
                    {
                        error!("Failed to store ML result: {}", e);
                    }

                    if ml_score > config_final.quarantine_threshold
                        && dashboard_state_final.should_emit_alert(scores.process_id, scores.timestamp)
                    {
                        info!("Ransomware detected! ML score: {:.2}", ml_score);

                        // Trigger quarantine
                        if let Err(e) = quarantine_final.quarantine_process(scores.process_id).await {
                            error!("Quarantine failed: {}", e);
                        }

                        let triggered_detectors = risk_snapshots
                            .iter()
                            .find(|risk| risk.process_id == scores.process_id)
                            .map(|risk| risk.active_detectors.clone())
                            .unwrap_or_default();

                        dashboard_state_final.add_alert(grpc_server::sentinelguard::Alert {
                            id: chrono::Utc::now().timestamp_millis(),
                            process_id: scores.process_id,
                            process_path: scores.process_path.clone(),
                            ml_score: ml_score as f64,
                            quarantined: true,
                            timestamp: scores.timestamp,
                            triggered_detectors,
                        });

                        // Log alert
                        if let Err(e) = db_final.log_alert(&scores, ml_score).await {
                            error!("Failed to log alert: {}", e);
                        }
                    }
                }
            }
        }
    });

    // Start gRPC server for UI communication
    let db_grpc = db.clone();
    let quarantine_grpc = quarantine.clone();
    let grpc_addr = config
        .grpc_listen_addr
        .parse()
        .unwrap_or_else(|_| "127.0.0.1:50051".parse().unwrap());

    tokio::spawn(async move {
        if let Err(e) = start_grpc_server(db_grpc, quarantine_grpc, dashboard_state, grpc_addr).await {
            error!("gRPC server error: {}", e);
        }
    });
    info!("gRPC server started on {}", grpc_addr);

    info!("SentinelGuard Agent running. Press Ctrl+C to stop.");

    wait_for_shutdown(shutdown_mode).await?;
    info!("Shutting down...");

    Ok(())
}

async fn wait_for_shutdown(shutdown_mode: ShutdownMode) -> Result<()> {
    match shutdown_mode {
        ShutdownMode::Console => {
            tokio::signal::ctrl_c().await?;
        }
        ShutdownMode::Service(mut shutdown_rx) => {
            if !*shutdown_rx.borrow() {
                let _ = shutdown_rx.changed().await;
            }
        }
    }

    Ok(())
}
