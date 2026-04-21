//! SentinelGuard User-Mode Agent
//!
//! Main entry point for the ransomware detection agent.
//! Orchestrates all modules: driver communication, event processing,
//! detection, correlation, ML inference, quarantine, database persistence,
//! and the gRPC API.

mod communication;
mod config;
mod correlation;
mod database;
mod detectors;
mod events;
mod grpc_server;
mod inference;
mod quarantine;
mod security;
mod telemetry;

use crate::communication::DriverConnection;
use crate::config::AppConfig;
use crate::correlation::Correlator;
use crate::database::Database;
use crate::detectors::DetectorRegistry;
use crate::events::{Alert, QuarantineStatus, Severity};
use crate::grpc_server::{start_grpc_server, ServiceState};
use crate::inference::InferenceEngine;
use crate::quarantine::QuarantineManager;
use crate::telemetry::{init_telemetry, Metrics};

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{debug, error, info, warn};

/// Minimum heuristic score to trigger ML inference.
/// Events scoring below this skip the expensive ONNX model entirely.
const ML_INFERENCE_THRESHOLD: f64 = 0.1;

/// Deduplication window: skip identical (pid, path, op) events within this period.
const DEDUP_WINDOW_NS: u64 = 50_000_000; // 50ms

const DEFAULT_CONFIG_PATH: &str = r"C:\ProgramData\SentinelGuard\config.toml";

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CONFIG_PATH.to_string());

    let config = AppConfig::load(&PathBuf::from(&config_path))
        .with_context(|| format!("Failed to load config from {}", config_path))?;

    // Initialize telemetry (logging)
    let _log_guard = init_telemetry(&config.telemetry, &config.agent.log_level)
        .context("Failed to initialize telemetry")?;

    info!(
        "SentinelGuard Agent v{} starting",
        config.agent.version
    );

    // Verify security constraints
    if !security::is_localhost_address(&config.grpc.listen_addr) {
        warn!(
            "gRPC listen address {} is not localhost. This is a security risk.",
            config.grpc.listen_addr
        );
    }

    // Initialize metrics
    let metrics = Arc::new(Metrics::new());

    // Initialize database
    let database = Arc::new(
        Database::open(&config.database.path, config.database.wal_mode)
            .context("Failed to open database")?,
    );

    // Initialize detector registry
    let detector_registry = Arc::new(DetectorRegistry::new(&config.detectors));
    info!(
        "Initialized detectors: {:?}",
        detector_registry.detector_names()
    );

    // Initialize correlator
    let correlator = Arc::new(Correlator::new(
        config.detectors.weights.clone(),
        config.detectors.window_seconds,
    ));

    // Initialize inference engine
    let inference_engine = Arc::new(
        InferenceEngine::new(&config.inference)
            .context("Failed to initialize inference engine")?,
    );
    let model_loaded = Arc::new(AtomicBool::new(inference_engine.is_model_loaded()));

    // Initialize quarantine manager
    let quarantine_manager = Arc::new(QuarantineManager::new(&config.quarantine));
    if !quarantine_manager.is_available() {
        warn!(
            "Quarantine helper not found at {}. Quarantine actions will fail.",
            config.quarantine.helper_path
        );
    }

    // Initialize driver connection
    let driver_connection = Arc::new(DriverConnection::new());
    let driver_connected = Arc::new(AtomicBool::new(false));

    // Alert broadcasting channel for gRPC streaming
    let (alert_tx, _) = broadcast::channel::<grpc_server::proto::Alert>(1000);

    // Shutdown coordination
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Event channel from driver to processing pipeline
    let (event_tx, mut event_rx) =
        mpsc::channel(config.agent.event_buffer_size);

    // Start driver communication
    let dc_shutdown = shutdown_rx.clone();
    let dc = driver_connection.clone();
    let _dc_connected = driver_connected.clone();
    tokio::spawn(async move {
        if let Err(e) = dc.start_receiving(
            config.driver.port_name.clone(),
            event_tx,
            dc_shutdown,
        ).await {
            error!("Driver communication failed: {}", e);
        }
    });

    // Monitor driver connection state
    let dc_monitor = driver_connection.clone();
    let dc_conn_flag = driver_connected.clone();
    let mut monitor_shutdown = shutdown_rx.clone();
    tokio::spawn(async move {
        loop {
            dc_conn_flag.store(dc_monitor.is_connected(), Ordering::Relaxed);
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {}
                _ = monitor_shutdown.changed() => break,
            }
        }
    });

    // Build gRPC service state
    let service_state = Arc::new(ServiceState {
        database: database.clone(),
        correlator: correlator.clone(),
        quarantine: quarantine_manager.clone(),
        metrics: metrics.clone(),
        driver_connected: driver_connected.clone(),
        model_loaded: model_loaded.clone(),
        alert_broadcaster: alert_tx.clone(),
        agent_version: config.agent.version.clone(),
    });

    // Start gRPC server
    let grpc_addr = config.grpc.listen_addr.clone();
    let grpc_state = service_state.clone();
    tokio::spawn(async move {
        if let Err(e) = start_grpc_server(&grpc_addr, grpc_state).await {
            error!("gRPC server failed: {}", e);
        }
    });

    info!("gRPC server starting on {}", config.grpc.listen_addr);

    // Quarantine threshold
    let quarantine_threshold = config.quarantine.auto_quarantine_threshold;

    // Setup dedicated database writer thread to prevent blocking OS I/O.
    // FIX: use proper blocking recv instead of busy-spin (try_recv + sleep).
    let (db_tx, mut db_rx) = mpsc::channel(100_000);
    let writer_db = database.clone();
    let mut db_shutdown = shutdown_rx.clone();
    let db_rt_handle = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        loop {
            // Block until an event arrives OR the channel closes.
            // This is the proper way to drain an mpsc from a blocking thread.
            match db_rt_handle.block_on(async {
                tokio::select! {
                    event = db_rx.recv() => event,
                    _ = db_shutdown.changed() => None,
                }
            }) {
                Some(event) => {
                    if let Err(e) = writer_db.insert_event(&event) {
                        warn!("Failed to persist event (background): {:#}", e);
                    }
                    // Drain any additional buffered events without re-entering async
                    while let Ok(event) = db_rx.try_recv() {
                        if let Err(e) = writer_db.insert_event(&event) {
                            warn!("Failed to persist event (background): {:#}", e);
                        }
                    }
                }
                None => break, // channel closed or shutdown
            }
        }
    });

    // Main event processing loop
    let processing_metrics = metrics.clone();
    let processing_db = database.clone();
    let processing_det = detector_registry.clone();
    let processing_corr = correlator.clone();
    let processing_inf = inference_engine.clone();
    let processing_qm = quarantine_manager.clone();
    let processing_alert_tx = alert_tx.clone();
    let processing_metrics_alerts = metrics.clone();

    // Build whitelist set for fast lookups (lowercased)
    let process_whitelist: std::collections::HashSet<String> = config
        .agent
        .process_whitelist
        .iter()
        .map(|p| p.to_lowercase())
        .collect();
    if !process_whitelist.is_empty() {
        info!("Process whitelist active: {:?}", process_whitelist);
    }

    info!("Starting event processing pipeline");

    let mut process_shutdown = shutdown_rx.clone();

    tokio::spawn(async move {
        // Event deduplication map: (process_id, file_path_hash, operation) -> last_timestamp_ns
        let mut dedup_map: HashMap<(u32, u64, u32), u64> = HashMap::new();
        // Alert deduplication map to prevent spamming the UI for the same process
        let mut alerted_pids: HashMap<u32, u64> = HashMap::new();
        let mut dedup_cleanup_counter: u64 = 0;

        loop {
            tokio::select! {
                Some(event) = event_rx.recv() => {
                    // Validate event
                    if let Err(e) = event.validate() {
                        warn!("Invalid event: {}", e);
                        processing_metrics.increment_events_dropped();
                        continue;
                    }

                    // Skip whitelisted processes entirely to avoid false positives
                    // from known-safe system processes (svchost, Defender, etc.)
                    if !process_whitelist.is_empty() {
                        let exe_name = event.process_name
                            .rsplit('\\')
                            .next()
                            .unwrap_or(&event.process_name)
                            .to_lowercase();
                        if process_whitelist.contains(&exe_name) {
                            processing_metrics.increment_events_dropped();
                            continue;
                        }
                    }

                    // Event deduplication: skip identical (pid, path, op) within 50ms.
                    // This collapses write-storms from apps that update the same file rapidly.
                    let path_hash = {
                        use std::hash::{Hash, Hasher};
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        event.file_path.hash(&mut hasher);
                        hasher.finish()
                    };
                    let dedup_key = (event.process_id, path_hash, event.operation as u32);
                    if let Some(&last_ts) = dedup_map.get(&dedup_key) {
                        if event.timestamp_ns.saturating_sub(last_ts) < DEDUP_WINDOW_NS {
                            processing_metrics.increment_events_dropped();
                            debug!("Dedup: skipping rapid duplicate event for PID {} on {}",
                                event.process_id, event.file_path);
                            continue;
                        }
                    }
                    dedup_map.insert(dedup_key, event.timestamp_ns);

                    // Periodic cleanup of dedup map to prevent unbounded growth
                    dedup_cleanup_counter += 1;
                    if dedup_cleanup_counter % 10_000 == 0 {
                        let cutoff = event.timestamp_ns.saturating_sub(DEDUP_WINDOW_NS * 20);
                        dedup_map.retain(|_, ts| *ts > cutoff);

                        let alert_cutoff = event.timestamp_ns.saturating_sub(60_000_000_000); // 60s
                        alerted_pids.retain(|_, ts| *ts > alert_cutoff);
                    }

                    processing_metrics.increment_events_processed();

                    // Send to dedicated DB writer without blocking
                    let _ = db_tx.try_send(event.clone());

                    // Run detectors
                    let detector_results = processing_det.evaluate_all(&event);

                    // Aggregate results
                    let mut aggregated = processing_corr.add_results(
                        event.process_id,
                        &event.process_name,
                        detector_results,
                    );

                    // Conditional ML inference: run the ONNX model when:
                    //  1. Heuristic detectors found something suspicious, OR
                    //  2. There's a stale high ML score that needs re-evaluation
                    //     (detectors dropped but old ml_score is still cached).
                    // Case 2 ensures the model can clear its own false positives.
                    let needs_inference = aggregated.weighted_score > ML_INFERENCE_THRESHOLD
                        || aggregated.ml_score > 0.05;
                    if needs_inference {
                        if let Err(e) = processing_inf.predict(&mut aggregated) {
                            warn!("ML inference error: {}", e);
                        } else {
                            // Write the ML results back to the Correlator so the API can serve them
                            processing_corr.update_scores(
                                event.process_id,
                                aggregated.ml_score,
                                aggregated.final_score,
                            );
                        }
                    }

                    // Check if quarantine threshold is exceeded
                    if aggregated.final_score >= quarantine_threshold {
                        let now_ns = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos() as u64;

                        // Only generate one alert per minute per PID to prevent UI/DB spam
                        let should_alert = match alerted_pids.get(&event.process_id) {
                            Some(&last_alert_ns) => now_ns.saturating_sub(last_alert_ns) > 60_000_000_000,
                            None => true,
                        };

                        if should_alert {
                            alerted_pids.insert(event.process_id, now_ns);
                        let severity = Severity::from(aggregated.final_score);

                        let alert = Alert {
                            alert_id: 0, // Auto-incremented by DB
                            process_id: event.process_id,
                            process_name: event.process_name.clone(),
                            severity,
                            risk_score: aggregated.final_score,
                            description: format!(
                                "Ransomware-like behavior detected from {} (PID {}) with risk score {:.2}",
                                event.process_name, event.process_id, aggregated.final_score
                            ),
                            detector_results: aggregated.detector_results.clone(),
                            quarantine_status: QuarantineStatus::Unknown,
                            timestamp_ns: now_ns,
                        };

                        // Persist alert
                        let db = processing_db.clone();
                        let alert_for_db = alert.clone();
                        if let Ok(alert_id) = tokio::task::spawn_blocking(move || {
                            db.insert_alert(&alert_for_db)
                        }).await.unwrap_or(Err(anyhow::anyhow!("Join error"))) {
                            info!("Alert {} generated for PID {}", alert_id, event.process_id);
                        }

                        processing_metrics_alerts.increment_alerts_generated();

                        // Broadcast alert for SSE streaming
                        let proto_alert = grpc_server::proto::Alert {
                            alert_id: 0,
                            process_id: event.process_id,
                            process_name: event.process_name.clone(),
                            severity: severity as i32,
                            risk_score: aggregated.final_score,
                            description: alert.description.clone(),
                            detector_results: aggregated.detector_results.iter().map(|r| {
                                grpc_server::proto::DetectorResult {
                                    detector_name: r.detector_name.clone(),
                                    score: r.score,
                                    evidence: r.evidence.clone(),
                                    timestamp_ns: r.timestamp_ns,
                                    process_id: r.process_id,
                                }
                            }).collect(),
                            quarantine_status: 0,
                            timestamp_ns: now_ns,
                        };
                        let _ = processing_alert_tx.send(proto_alert);

                        // Quarantine the process
                        let qm = processing_qm.clone();
                        let qdb = processing_db.clone();
                        let pname = event.process_name.clone();
                        let risk = aggregated.final_score;
                        let pid = event.process_id;
                        tokio::task::spawn_blocking(move || {
                            match qm.suspend_process(pid) {
                                Ok(result) => {
                                    if result.success {
                                        let _ = qdb.log_quarantine(
                                            pid, &pname, risk,
                                            "suspend", QuarantineStatus::Suspended,
                                        );
                                        info!("Process {} quarantined", pid);
                                    } else {
                                        warn!("Quarantine failed for PID {}: {}", pid, result.message);
                                    }
                                }
                                Err(e) => {
                                    error!("Quarantine error for PID {}: {}", pid, e);
                                }
                            }
                        });
                        }
                    }
                }
                _ = process_shutdown.changed() => {
                    info!("Event processing shutting down");
                    break;
                }
            }
        }
    });

    // Periodic cleanup task
    let cleanup_corr = correlator.clone();
    let mut cleanup_shutdown = shutdown_rx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
                    cleanup_corr.cleanup_expired();
                }
                _ = cleanup_shutdown.changed() => break,
            }
        }
    });

    // Wait for shutdown signal (Ctrl+C)
    info!("SentinelGuard agent is running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c()
        .await
        .context("Failed to listen for Ctrl+C")?;

    info!("Shutdown signal received, stopping...");
    let _ = shutdown_tx.send(true);

    // Give tasks time to clean up
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!(
        "SentinelGuard agent stopped. Events processed: {}, Alerts generated: {}",
        metrics.events_processed(),
        metrics.alerts_generated()
    );

    Ok(())
}
