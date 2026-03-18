//! SentinelGuard Telemetry Module
//!
//! Configures structured logging with tracing-subscriber,
//! supporting both console and file output with rotation.

use crate::config::TelemetryConfig;
use anyhow::{Context, Result};
use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the telemetry/logging system.
/// Returns a guard that must be held for the lifetime of the application
/// to ensure log flushing.
pub fn init_telemetry(config: &TelemetryConfig, log_level: &str) -> Result<WorkerGuard> {
    // Ensure log directory exists
    let log_path = Path::new(&config.log_file);
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create log directory: {}", parent.display()))?;
    }

    let log_dir = log_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let log_filename = log_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("sentinelguard.log"));

    // Create the file appender with rotation
    let file_appender = tracing_appender::rolling::daily(log_dir, log_filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Build the subscriber with env filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json()
                .with_writer(non_blocking),
        )
        .with(
            fmt::layer()
                .with_target(false)
                .compact()
                .with_writer(std::io::stderr),
        )
        .init();

    Ok(guard)
}

/// Metrics counters for operational telemetry
pub struct Metrics {
    events_processed: std::sync::atomic::AtomicU64,
    alerts_generated: std::sync::atomic::AtomicU64,
    events_dropped: std::sync::atomic::AtomicU64,
    start_time: std::time::Instant,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            events_processed: std::sync::atomic::AtomicU64::new(0),
            alerts_generated: std::sync::atomic::AtomicU64::new(0),
            events_dropped: std::sync::atomic::AtomicU64::new(0),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn increment_events_processed(&self) {
        self.events_processed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn increment_alerts_generated(&self) {
        self.alerts_generated
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn increment_events_dropped(&self) {
        self.events_dropped
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn events_processed(&self) -> u64 {
        self.events_processed
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn alerts_generated(&self) -> u64 {
        self.alerts_generated
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub fn events_per_second(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_secs().max(1);
        self.events_processed() / elapsed
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
