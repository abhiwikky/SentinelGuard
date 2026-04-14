//! SentinelGuard Configuration Module
//!
//! Loads and validates configuration from a TOML file.
//! Default path: %ProgramData%\SentinelGuard\config.toml

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Top-level configuration structure
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub agent: AgentConfig,
    pub driver: DriverConfig,
    pub grpc: GrpcConfig,
    pub database: DatabaseConfig,
    pub detectors: DetectorsConfig,
    pub quarantine: QuarantineConfig,
    pub inference: InferenceConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub version: String,
    pub log_level: String,
    pub event_buffer_size: usize,
    pub health_report_interval_secs: u64,
    /// Processes to skip during detection (known-safe system processes).
    /// Matched case-insensitively against the executable name only.
    #[serde(default)]
    pub process_whitelist: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DriverConfig {
    pub port_name: String,
    pub max_connections: u32,
    pub max_message_size: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrpcConfig {
    pub listen_addr: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub wal_mode: bool,
    pub max_size_mb: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DetectorsConfig {
    pub window_seconds: u64,
    pub weights: DetectorWeights,
    pub entropy: EntropyConfig,
    pub mass_write: MassWriteConfig,
    pub mass_rename_delete: MassRenameDeleteConfig,
    pub ransom_note: RansomNoteConfig,
    pub shadow_copy: ShadowCopyConfig,
    pub process_behavior: ProcessBehaviorConfig,
    pub extension_explosion: ExtensionExplosionConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DetectorWeights {
    pub entropy_spike: f64,
    pub mass_write: f64,
    pub mass_rename_delete: f64,
    pub ransom_note: f64,
    pub shadow_copy: f64,
    pub process_behavior: f64,
    pub extension_explosion: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntropyConfig {
    pub threshold: f64,
    pub min_file_size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MassWriteConfig {
    pub count_threshold: u64,
    pub window_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MassRenameDeleteConfig {
    pub count_threshold: u64,
    pub window_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RansomNoteConfig {
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShadowCopyConfig {
    pub suspicious_processes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProcessBehaviorConfig {
    pub max_extensions: usize,
    pub max_directories: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtensionExplosionConfig {
    pub new_extension_threshold: usize,
    pub window_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuarantineConfig {
    pub helper_path: String,
    pub auto_quarantine_threshold: f64,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InferenceConfig {
    pub model_path: String,
    pub num_features: usize,
    pub fallback_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryConfig {
    pub log_file: String,
    pub max_log_size_mb: u64,
    pub max_log_files: u32,
}

impl AppConfig {
    /// Load configuration from the specified TOML file path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: AppConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        config.validate()?;
        Ok(config)
    }

    /// Load from the default system path.
    pub fn load_default() -> Result<Self> {
        let default_path = Self::default_path();
        Self::load(&default_path)
    }

    /// Returns the default configuration file path.
    pub fn default_path() -> PathBuf {
        PathBuf::from(r"C:\ProgramData\SentinelGuard\config.toml")
    }

    /// Validate configuration values.
    fn validate(&self) -> Result<()> {
        if self.quarantine.auto_quarantine_threshold < 0.0
            || self.quarantine.auto_quarantine_threshold > 1.0
        {
            anyhow::bail!(
                "auto_quarantine_threshold must be between 0.0 and 1.0, got {}",
                self.quarantine.auto_quarantine_threshold
            );
        }

        if self.detectors.window_seconds == 0 {
            anyhow::bail!("detectors.window_seconds must be > 0");
        }

        // Validate weights sum to roughly 1.0
        let w = &self.detectors.weights;
        let total = w.entropy_spike
            + w.mass_write
            + w.mass_rename_delete
            + w.ransom_note
            + w.shadow_copy
            + w.process_behavior
            + w.extension_explosion;

        if (total - 1.0).abs() > 0.01 {
            tracing::warn!(
                "Detector weights sum to {:.4}, expected ~1.0. Scores will still be normalized.",
                total
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sample_toml() -> &'static str {
        r#"
[agent]
version = "1.0.0"
log_level = "info"
event_buffer_size = 10000
health_report_interval_secs = 30
process_whitelist = ["svchost.exe", "MsMpEng.exe", "SearchIndexer.exe", "csrss.exe", "lsass.exe", "services.exe", "smss.exe", "System"]

[driver]
port_name = "\\SentinelGuardPort"
max_connections = 1
max_message_size = 65536

[grpc]
listen_addr = "127.0.0.1:50051"

[database]
path = "C:\\ProgramData\\SentinelGuard\\sentinelguard.db"
wal_mode = true
max_size_mb = 0

[detectors]
window_seconds = 60
[detectors.weights]
entropy_spike = 0.20
mass_write = 0.15
mass_rename_delete = 0.15
ransom_note = 0.15
shadow_copy = 0.10
process_behavior = 0.15
extension_explosion = 0.10

[detectors.entropy]
threshold = 7.0
min_file_size = 1024

[detectors.mass_write]
count_threshold = 50
window_seconds = 10

[detectors.mass_rename_delete]
count_threshold = 20
window_seconds = 10

[detectors.ransom_note]
patterns = ["readme.txt", "how_to_decrypt"]

[detectors.shadow_copy]
suspicious_processes = ["vssadmin.exe", "wmic.exe"]

[detectors.process_behavior]
max_extensions = 15
max_directories = 50

[detectors.extension_explosion]
new_extension_threshold = 10
window_seconds = 30

[quarantine]
helper_path = "C:\\Program Files\\SentinelGuard\\quarantine_helper.exe"
auto_quarantine_threshold = 0.75
timeout_seconds = 10

[inference]
model_path = "C:\\ProgramData\\SentinelGuard\\model.onnx"
num_features = 7
fallback_enabled = true

[telemetry]
log_file = "C:\\ProgramData\\SentinelGuard\\logs\\sentinelguard.log"
max_log_size_mb = 100
max_log_files = 5
"#
    }

    #[test]
    fn test_parse_config() {
        let config: AppConfig = toml::from_str(sample_toml()).unwrap();
        assert_eq!(config.agent.version, "1.0.0");
        assert_eq!(config.grpc.listen_addr, "127.0.0.1:50051");
        assert!((config.quarantine.auto_quarantine_threshold - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_config_validation() {
        let mut config: AppConfig = toml::from_str(sample_toml()).unwrap();
        config.quarantine.auto_quarantine_threshold = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_load_from_file() {
        let dir = std::env::temp_dir().join("sg_test_config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(sample_toml().as_bytes()).unwrap();
        drop(f);

        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.agent.log_level, "info");

        std::fs::remove_dir_all(&dir).ok();
    }
}
