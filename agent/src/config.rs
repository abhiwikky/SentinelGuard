//
// Configuration management
//

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub ml_model_path: PathBuf,
    pub quarantine_path: PathBuf,
    pub quarantine_threshold: f32,
    pub grpc_listen_addr: String,
    pub detector_config: DetectorConfig,
    pub communication: CommunicationConfig,
    pub config_path: Option<PathBuf>,
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

#[derive(Debug, Deserialize)]
struct RawConfig {
    database: Option<RawDatabaseConfig>,
    ml: Option<RawMlConfig>,
    quarantine: Option<RawQuarantineConfig>,
    detectors: Option<RawDetectorConfig>,
    communication: Option<RawCommunicationConfig>,
    grpc: Option<RawGrpcConfig>,
}

#[derive(Debug, Deserialize)]
struct RawDatabaseConfig {
    path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct RawMlConfig {
    model_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct RawQuarantineConfig {
    path: Option<PathBuf>,
    threshold: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct RawDetectorConfig {
    entropy_threshold: Option<f32>,
    mass_write_threshold: Option<usize>,
    mass_write_window_seconds: Option<u64>,
    rename_delete_threshold: Option<usize>,
    rename_delete_window_seconds: Option<u64>,
    ransom_note_patterns: Option<Vec<String>>,
    yara_rules_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct RawCommunicationConfig {
    port_name: Option<String>,
    buffer_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RawGrpcConfig {
    listen_addr: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let default_config = Self::default();
        let config_path = Self::discover_config_path();

        let Some(config_path) = config_path else {
            return Ok(default_config);
        };

        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {}", config_path.display()))?;
        let parsed: RawConfig = toml::from_str(&raw)
            .with_context(|| format!("Failed to parse TOML from {}", config_path.display()))?;
        let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        let search_roots = build_search_roots(config_dir);

        Ok(Config {
            database_path: parsed
                .database
                .and_then(|database| database.path)
                .map(|path| resolve_path(&search_roots, &path))
                .unwrap_or(default_config.database_path),
            ml_model_path: resolve_model_path(
                &search_roots,
                parsed.ml.and_then(|ml| ml.model_path),
                &default_config.ml_model_path,
            ),
            quarantine_path: parsed
                .quarantine
                .as_ref()
                .and_then(|quarantine| quarantine.path.as_ref())
                .map(|path| resolve_path(&search_roots, path))
                .unwrap_or(default_config.quarantine_path),
            quarantine_threshold: parsed
                .quarantine
                .and_then(|quarantine| quarantine.threshold)
                .unwrap_or(default_config.quarantine_threshold),
            grpc_listen_addr: parsed
                .grpc
                .and_then(|grpc| grpc.listen_addr)
                .unwrap_or(default_config.grpc_listen_addr),
            detector_config: merge_detector_config(
                parsed.detectors,
                &search_roots,
                default_config.detector_config,
            ),
            communication: merge_communication_config(
                parsed.communication,
                default_config.communication,
            ),
            config_path: Some(config_path),
        })
    }

    pub fn default() -> Self {
        Self {
            database_path: PathBuf::from("C:\\ProgramData\\SentinelGuard\\sentinelguard.db"),
            ml_model_path: PathBuf::from("models\\sentinelguard_model.onnx"),
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
            config_path: None,
        }
    }

    fn discover_config_path() -> Option<PathBuf> {
        let mut candidates = Vec::new();

        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                candidates.push(exe_dir.join("config").join("config.toml"));
                candidates.push(exe_dir.join("agent").join("config").join("config.toml"));
            }
        }

        if let Ok(current_dir) = env::current_dir() {
            candidates.push(current_dir.join("config").join("config.toml"));
            candidates.push(current_dir.join("agent").join("config").join("config.toml"));
        }

        candidates.into_iter().find(|candidate| candidate.exists())
    }
}

fn merge_detector_config(
    raw: Option<RawDetectorConfig>,
    search_roots: &[PathBuf],
    default_config: DetectorConfig,
) -> DetectorConfig {
    let raw = raw.unwrap_or(RawDetectorConfig {
        entropy_threshold: None,
        mass_write_threshold: None,
        mass_write_window_seconds: None,
        rename_delete_threshold: None,
        rename_delete_window_seconds: None,
        ransom_note_patterns: None,
        yara_rules_path: None,
    });

    DetectorConfig {
        entropy_threshold: raw.entropy_threshold.unwrap_or(default_config.entropy_threshold),
        mass_write_threshold: raw.mass_write_threshold.unwrap_or(default_config.mass_write_threshold),
        mass_write_window_seconds: raw
            .mass_write_window_seconds
            .unwrap_or(default_config.mass_write_window_seconds),
        rename_delete_threshold: raw
            .rename_delete_threshold
            .unwrap_or(default_config.rename_delete_threshold),
        rename_delete_window_seconds: raw
            .rename_delete_window_seconds
            .unwrap_or(default_config.rename_delete_window_seconds),
        ransom_note_patterns: raw
            .ransom_note_patterns
            .unwrap_or(default_config.ransom_note_patterns),
        yara_rules_path: raw
            .yara_rules_path
            .map(|path| resolve_path(search_roots, &path))
            .unwrap_or(default_config.yara_rules_path),
    }
}

fn merge_communication_config(
    raw: Option<RawCommunicationConfig>,
    default_config: CommunicationConfig,
) -> CommunicationConfig {
    let raw = raw.unwrap_or(RawCommunicationConfig {
        port_name: None,
        buffer_size: None,
    });

    CommunicationConfig {
        port_name: raw.port_name.unwrap_or(default_config.port_name),
        buffer_size: raw.buffer_size.unwrap_or(default_config.buffer_size),
    }
}

fn resolve_model_path(
    search_roots: &[PathBuf],
    configured_path: Option<PathBuf>,
    default_path: &Path,
) -> PathBuf {
    let requested_path = configured_path.unwrap_or_else(|| default_path.to_path_buf());
    let requested_file_name = requested_path
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("sentinelguard_model.onnx"));

    for candidate in build_path_candidates(search_roots, &requested_path) {
        if candidate.exists() {
            return candidate;
        }
    }

    let fallback_names = if requested_file_name
        .to_string_lossy()
        .eq_ignore_ascii_case("ransomware_model.onnx")
    {
        vec![PathBuf::from("sentinelguard_model.onnx")]
    } else {
        vec![PathBuf::from("sentinelguard_model.onnx"), PathBuf::from("ransomware_model.onnx")]
    };

    for fallback_name in fallback_names {
        for root in search_roots {
            let direct = root.join(&fallback_name);
            if direct.exists() {
                return direct;
            }

            let models = root.join("models").join(&fallback_name);
            if models.exists() {
                return models;
            }

            let repo_models = root.join("ml").join("models").join(&fallback_name);
            if repo_models.exists() {
                return repo_models;
            }
        }
    }

    resolve_path(search_roots, &requested_path)
}

fn build_search_roots(config_dir: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    roots.push(config_dir.to_path_buf());

    if config_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("config"))
        .unwrap_or(false)
    {
        if let Some(parent) = config_dir.parent() {
            roots.push(parent.to_path_buf());
        }
    }

    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            roots.push(exe_dir.to_path_buf());
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        roots.push(current_dir);
    }

    roots.dedup();
    roots
}

fn build_path_candidates(search_roots: &[PathBuf], path: &Path) -> Vec<PathBuf> {
    if path.is_absolute() {
        return vec![path.to_path_buf()];
    }

    let mut candidates = Vec::new();
    let file_name = path.file_name().map(PathBuf::from);

    for root in search_roots {
        candidates.push(root.join(path));

        if let Some(file_name) = &file_name {
            candidates.push(root.join("models").join(file_name));
            candidates.push(root.join("ml").join("models").join(file_name));
        }
    }

    candidates
}

fn resolve_path(search_roots: &[PathBuf], path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        for candidate in build_path_candidates(search_roots, path) {
            if candidate.exists() {
                return candidate;
            }
        }

        search_roots
            .first()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(path)
    }
}
