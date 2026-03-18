//! SentinelGuard Event Definitions
//!
//! Normalized event structures used throughout the agent after
//! ingestion from the kernel driver.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Operation types matching the kernel driver enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum OperationType {
    Unknown = 0,
    Create = 1,
    Write = 2,
    Read = 3,
    Rename = 4,
    Delete = 5,
    DirectoryEnum = 6,
    ShadowCopyDelete = 7,
}

impl From<u32> for OperationType {
    fn from(val: u32) -> Self {
        match val {
            1 => OperationType::Create,
            2 => OperationType::Write,
            3 => OperationType::Read,
            4 => OperationType::Rename,
            5 => OperationType::Delete,
            6 => OperationType::DirectoryEnum,
            7 => OperationType::ShadowCopyDelete,
            _ => OperationType::Unknown,
        }
    }
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationType::Unknown => write!(f, "Unknown"),
            OperationType::Create => write!(f, "Create"),
            OperationType::Write => write!(f, "Write"),
            OperationType::Read => write!(f, "Read"),
            OperationType::Rename => write!(f, "Rename"),
            OperationType::Delete => write!(f, "Delete"),
            OperationType::DirectoryEnum => write!(f, "DirectoryEnum"),
            OperationType::ShadowCopyDelete => write!(f, "ShadowCopyDelete"),
        }
    }
}

/// Normalized file event after ingestion from kernel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub event_id: u64,
    pub process_id: u32,
    pub process_name: String,
    pub operation: OperationType,
    pub file_path: String,
    pub new_file_path: Option<String>,
    pub file_size: u64,
    pub entropy: f64,
    pub timestamp_ns: u64,
    pub file_extension: String,
}

impl FileEvent {
    /// Create a new event with the current timestamp
    pub fn new(
        event_id: u64,
        process_id: u32,
        process_name: String,
        operation: OperationType,
        file_path: String,
    ) -> Self {
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let file_extension = file_path
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();

        Self {
            event_id,
            process_id,
            process_name,
            operation,
            file_path,
            new_file_path: None,
            file_size: 0,
            entropy: 0.0,
            timestamp_ns,
            file_extension,
        }
    }

    /// Validate the event for required fields
    pub fn validate(&self) -> Result<(), EventValidationError> {
        if self.process_id == 0 {
            return Err(EventValidationError::InvalidProcessId);
        }
        if self.file_path.is_empty() {
            return Err(EventValidationError::EmptyFilePath);
        }
        if self.timestamp_ns == 0 {
            return Err(EventValidationError::InvalidTimestamp);
        }
        Ok(())
    }

    /// Get the directory containing this file
    pub fn directory(&self) -> &str {
        self.file_path
            .rfind('\\')
            .map(|i| &self.file_path[..i])
            .unwrap_or(&self.file_path)
    }

    /// Get the filename without directory
    pub fn filename(&self) -> &str {
        self.file_path
            .rfind('\\')
            .map(|i| &self.file_path[i + 1..])
            .unwrap_or(&self.file_path)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventValidationError {
    #[error("Process ID cannot be 0")]
    InvalidProcessId,
    #[error("File path cannot be empty")]
    EmptyFilePath,
    #[error("Timestamp cannot be 0")]
    InvalidTimestamp,
}

/// Detector result from a single detector evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorResult {
    pub detector_name: String,
    pub score: f64,
    pub evidence: Vec<String>,
    pub timestamp_ns: u64,
    pub process_id: u32,
}

impl DetectorResult {
    pub fn new(detector_name: &str, score: f64, evidence: Vec<String>, process_id: u32) -> Self {
        let score = score.clamp(0.0, 1.0);
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            detector_name: detector_name.to_string(),
            score,
            evidence,
            timestamp_ns,
            process_id,
        }
    }
}

/// Aggregated score for a process within a time window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedScore {
    pub process_id: u32,
    pub process_name: String,
    pub weighted_score: f64,
    pub ml_score: f64,
    pub final_score: f64,
    pub detector_results: Vec<DetectorResult>,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
}

/// Alert generated when risk threshold is exceeded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: u64,
    pub process_id: u32,
    pub process_name: String,
    pub severity: Severity,
    pub risk_score: f64,
    pub description: String,
    pub detector_results: Vec<DetectorResult>,
    pub quarantine_status: QuarantineStatus,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Unknown = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl From<f64> for Severity {
    fn from(score: f64) -> Self {
        if score >= 0.9 {
            Severity::Critical
        } else if score >= 0.75 {
            Severity::High
        } else if score >= 0.5 {
            Severity::Medium
        } else if score >= 0.25 {
            Severity::Low
        } else {
            Severity::Unknown
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuarantineStatus {
    Unknown = 0,
    Suspended = 1,
    Released = 2,
    ProcessExited = 3,
}

/// Health status of the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub agent_running: bool,
    pub driver_connected: bool,
    pub model_loaded: bool,
    pub database_connected: bool,
    pub events_processed: u64,
    pub alerts_generated: u64,
    pub uptime_seconds: u64,
    pub events_per_second: u64,
    pub agent_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_type_conversion() {
        assert_eq!(OperationType::from(1), OperationType::Create);
        assert_eq!(OperationType::from(5), OperationType::Delete);
        assert_eq!(OperationType::from(99), OperationType::Unknown);
    }

    #[test]
    fn test_file_event_creation() {
        let event = FileEvent::new(
            1,
            1234,
            "test.exe".to_string(),
            OperationType::Write,
            r"C:\Users\test\document.docx".to_string(),
        );
        assert_eq!(event.file_extension, "docx");
        assert!(event.timestamp_ns > 0);
    }

    #[test]
    fn test_event_validation() {
        let mut event = FileEvent::new(
            1,
            1234,
            "test.exe".to_string(),
            OperationType::Write,
            r"C:\test.txt".to_string(),
        );
        assert!(event.validate().is_ok());

        event.process_id = 0;
        assert!(event.validate().is_err());
    }

    #[test]
    fn test_file_event_directory() {
        let event = FileEvent::new(
            1,
            1234,
            "test.exe".to_string(),
            OperationType::Write,
            r"C:\Users\test\document.docx".to_string(),
        );
        assert_eq!(event.directory(), r"C:\Users\test");
        assert_eq!(event.filename(), "document.docx");
    }

    #[test]
    fn test_severity_from_score() {
        assert_eq!(Severity::from(0.95), Severity::Critical);
        assert_eq!(Severity::from(0.80), Severity::High);
        assert_eq!(Severity::from(0.55), Severity::Medium);
        assert_eq!(Severity::from(0.30), Severity::Low);
        assert_eq!(Severity::from(0.10), Severity::Unknown);
    }

    #[test]
    fn test_detector_result_clamp() {
        let result = DetectorResult::new("test", 1.5, vec![], 1234);
        assert!((result.score - 1.0).abs() < f64::EPSILON);

        let result2 = DetectorResult::new("test", -0.5, vec![], 1234);
        assert!((result2.score - 0.0).abs() < f64::EPSILON);
    }
}
