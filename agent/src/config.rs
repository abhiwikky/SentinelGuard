//
// Configuration management
//

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "toml")]
use toml;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub ml_model_path: PathBuf,
    pub quarantine_path: PathBuf,
    pub quarantine_threshold: f32,
    pub grpc_listen_addr: String,
    pub detector_config: DetectorConfig,
    pub communication: CommunicationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    pub entropy_threshold: f32,
    pub mass_write_threshold: usize,
    pub mass_write_window_seconds: u64,
    pub rename_delete_threshold: usize,
    pub rename_delete_window_seconds: u64,
    pub ransom_note_patterns: Vec<String>,
    pub yara_rules_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationConfig {
    pub port_name: String,
    pub buffer_size: usize,
}

impl Config {
    pub fn load() -> Result<Self> {
        // Try to load from config file, or use defaults
        let default_config = Config::default();
        
        // Try to load from config file, or use defaults
        // In production, would parse TOML here
        Ok(default_config)
    }

    pub fn default() -> Self {
        Config {
            database_path: PathBuf::from("C:\\ProgramData\\SentinelGuard\\sentinelguard.db"),
            ml_model_path: PathBuf::from("models\\ransomware_model.onnx"),
            quarantine_path: PathBuf::from("C:\\Program Files\\SentinelGuard\\quarantine.exe"),
            quarantine_threshold: 0.7,
            grpc_listen_addr: "127.0.0.1:50051".to_string(),
            detector_config: DetectorConfig {
                entropy_threshold: 0.8,
                mass_write_threshold: 50,
                mass_write_window_seconds: 10,
                rename_delete_threshold: 30,
                rename_delete_window_seconds: 10,
                ransom_note_patterns: vec![
                    "YOUR FILES HAVE BEEN ENCRYPTED".to_string(),
                    "PAY BITCOIN".to_string(),
                    ".locked".to_string(),
                    ".encrypted".to_string(),
                ],
                yara_rules_path: PathBuf::from("rules\\ransomware.yar"),
            },
            communication: CommunicationConfig {
                port_name: "\\SentinelGuardPort".to_string(),
                buffer_size: 4096,
            },
        }
    }
}

