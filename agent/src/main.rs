//
// SentinelGuard User-Mode Agent
// Main entry point
//

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
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
mod telemetry;

use communication::KernelCommunication;
use config::Config;
use correlation::CorrelationEngine;
use database::Database;
use detectors::DetectorManager;
use events::EventIngestion;
use grpc_server::start_grpc_server;
use quarantine::QuarantineController;
use security::SecurityModule;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("sentinelguard_agent=info")
        .init();

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
    let kernel_comm = KernelCommunication::new(event_tx.clone())?;
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
    let correlation_engine_clone = correlation_engine.clone();
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
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // Get aggregated scores from detectors
            let scores = detector_manager.get_aggregated_scores().await;

            // Run ML correlation
            if let Ok(ml_score) = correlation_engine_final.infer(&scores).await {
                if ml_score > config_final.quarantine_threshold {
                    info!("Ransomware detected! ML score: {:.2}", ml_score);

                    // Trigger quarantine
                    if let Err(e) = quarantine_final.quarantine_process(scores.process_id).await {
                        error!("Quarantine failed: {}", e);
                    }

                    // Log alert
                    if let Err(e) = db_final.log_alert(&scores, ml_score).await {
                        error!("Failed to log alert: {}", e);
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
        if let Err(e) = start_grpc_server(db_grpc, quarantine_grpc, grpc_addr).await {
            error!("gRPC server error: {}", e);
        }
    });
    info!("gRPC server started on {}", grpc_addr);

    info!("SentinelGuard Agent running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    Ok(())
}
