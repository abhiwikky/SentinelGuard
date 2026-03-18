//! Shadow Copy Deletion Detector
//!
//! Detects processes associated with shadow copy deletion
//! (e.g., vssadmin.exe, wmic.exe invocations).

use crate::config::ShadowCopyConfig;
use crate::detectors::{Detector, DetectorState};
use crate::events::{DetectorResult, FileEvent, OperationType};

pub struct ShadowCopyDetector {
    suspicious_processes: Vec<String>,
}

impl ShadowCopyDetector {
    pub fn new(config: &ShadowCopyConfig) -> Self {
        Self {
            suspicious_processes: config
                .suspicious_processes
                .iter()
                .map(|p| p.to_lowercase())
                .collect(),
        }
    }

    fn is_suspicious_process(&self, process_name: &str) -> bool {
        let lower = process_name.to_lowercase();
        // Check just the executable name (strip path)
        let exe_name = lower
            .rsplit('\\')
            .next()
            .unwrap_or(&lower);

        self.suspicious_processes
            .iter()
            .any(|p| exe_name == p.as_str())
    }
}

impl Detector for ShadowCopyDetector {
    fn name(&self) -> &str {
        "shadow_copy"
    }

    fn evaluate(&self, event: &FileEvent, state: &mut DetectorState) -> DetectorResult {
        // Check for shadow copy related operations
        if event.operation == OperationType::ShadowCopyDelete {
            // Direct shadow copy deletion detection from driver
            let count = state.increment_counter(self.name(), event.process_id);
            let score = (0.8 + (count as f64 - 1.0) * 0.1).min(1.0);

            let evidence = vec![
                format!("Shadow copy deletion detected"),
                format!("Process: {} (PID: {})", event.process_name, event.process_id),
                format!("File: {}", event.file_path),
            ];

            return DetectorResult::new(self.name(), score, evidence, event.process_id);
        }

        // Also flag suspicious process names performing file operations
        if self.is_suspicious_process(&event.process_name) {
            let count = state.increment_counter(self.name(), event.process_id);

            // Context-dependent scoring
            let score = match event.operation {
                OperationType::Delete => (0.5 + (count as f64 * 0.1)).min(0.9),
                OperationType::Create | OperationType::Write => {
                    (0.2 + (count as f64 * 0.05)).min(0.5)
                }
                _ => 0.1,
            };

            let evidence = vec![
                format!(
                    "Suspicious process '{}' performing {:?}",
                    event.process_name, event.operation
                ),
                format!("File: {}", event.file_path),
            ];

            return DetectorResult::new(self.name(), score, evidence, event.process_id);
        }

        DetectorResult::new(self.name(), 0.0, vec![], event.process_id)
    }

    fn reset_process_state(&self, state: &mut DetectorState, process_id: u32) {
        if let Some(map) = state.counters.get_mut(self.name()) {
            map.remove(&process_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_shadow_copy_op() {
        let config = ShadowCopyConfig {
            suspicious_processes: vec!["vssadmin.exe".into()],
        };
        let detector = ShadowCopyDetector::new(&config);
        let mut state = DetectorState::new();

        let event = FileEvent::new(
            1, 100, "vssadmin.exe".into(),
            OperationType::ShadowCopyDelete,
            r"C:\Windows\System32\something".into(),
        );
        let result = detector.evaluate(&event, &mut state);
        assert!(result.score >= 0.8);
    }

    #[test]
    fn test_flags_suspicious_process() {
        let config = ShadowCopyConfig {
            suspicious_processes: vec!["vssadmin.exe".into(), "wmic.exe".into()],
        };
        let detector = ShadowCopyDetector::new(&config);
        let mut state = DetectorState::new();

        let event = FileEvent::new(
            1, 100, r"C:\Windows\System32\wmic.exe".into(),
            OperationType::Delete,
            r"C:\SomeFile.txt".into(),
        );
        let result = detector.evaluate(&event, &mut state);
        assert!(result.score > 0.0);
    }

    #[test]
    fn test_ignores_normal_process() {
        let config = ShadowCopyConfig {
            suspicious_processes: vec!["vssadmin.exe".into()],
        };
        let detector = ShadowCopyDetector::new(&config);
        let mut state = DetectorState::new();

        let event = FileEvent::new(
            1, 100, "notepad.exe".into(),
            OperationType::Write,
            r"C:\test.txt".into(),
        );
        let result = detector.evaluate(&event, &mut state);
        assert!((result.score - 0.0).abs() < f64::EPSILON);
    }
}
