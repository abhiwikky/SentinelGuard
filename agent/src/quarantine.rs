//! SentinelGuard Quarantine Module
//!
//! Invokes the quarantine helper binary to suspend or release processes.

use crate::config::QuarantineConfig;
use anyhow::{Context, Result};
use std::process::Command;
use tracing::{error, info, warn};

/// Manages process quarantine via the external helper binary
pub struct QuarantineManager {
    helper_path: String,
    timeout_seconds: u64,
}

impl QuarantineManager {
    pub fn new(config: &QuarantineConfig) -> Self {
        Self {
            helper_path: config.helper_path.clone(),
            timeout_seconds: config.timeout_seconds,
        }
    }

    /// Suspend a process by PID
    pub fn suspend_process(&self, process_id: u32) -> Result<QuarantineResult> {
        info!("Suspending process PID={}", process_id);

        let output = Command::new(&self.helper_path)
            .args(["--suspend", &process_id.to_string()])
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute quarantine helper: {}",
                    self.helper_path
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            info!("Process {} suspended successfully", process_id);
            Ok(QuarantineResult {
                success: true,
                message: stdout.trim().to_string(),
                exit_code: 0,
            })
        } else {
            let exit_code = output.status.code().unwrap_or(-1);
            let msg = match exit_code {
                2 => format!("Process {} not found or already exited", process_id),
                3 => format!("Access denied suspending process {}", process_id),
                4 => format!("Process {} is already suspended", process_id),
                _ => format!(
                    "Failed to suspend process {}: {} {}",
                    process_id,
                    stdout.trim(),
                    stderr.trim()
                ),
            };
            warn!("{}", msg);
            Ok(QuarantineResult {
                success: false,
                message: msg,
                exit_code,
            })
        }
    }

    /// Release (resume) a previously suspended process
    pub fn release_process(&self, process_id: u32) -> Result<QuarantineResult> {
        info!("Releasing process PID={}", process_id);

        let output = Command::new(&self.helper_path)
            .args(["--release", &process_id.to_string()])
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute quarantine helper: {}",
                    self.helper_path
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            info!("Process {} released successfully", process_id);
            Ok(QuarantineResult {
                success: true,
                message: stdout.trim().to_string(),
                exit_code: 0,
            })
        } else {
            let exit_code = output.status.code().unwrap_or(-1);
            let msg = format!(
                "Failed to release process {}: {} {}",
                process_id,
                stdout.trim(),
                stderr.trim()
            );
            error!("{}", msg);
            Ok(QuarantineResult {
                success: false,
                message: msg,
                exit_code,
            })
        }
    }

    /// Check if the helper binary exists
    pub fn is_available(&self) -> bool {
        std::path::Path::new(&self.helper_path).exists()
    }
}

/// Result of a quarantine operation
#[derive(Debug, Clone)]
pub struct QuarantineResult {
    pub success: bool,
    pub message: String,
    pub exit_code: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quarantine_manager_creation() {
        let config = QuarantineConfig {
            helper_path: "nonexistent.exe".to_string(),
            auto_quarantine_threshold: 0.75,
            timeout_seconds: 10,
        };
        let manager = QuarantineManager::new(&config);
        assert!(!manager.is_available());
    }
}
