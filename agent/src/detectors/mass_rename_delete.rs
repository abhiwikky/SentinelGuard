//! Mass Rename/Delete Detector
//!
//! Detects processes performing rapid rename and delete operations,
//! characteristic of ransomware that renames files with new extensions
//! and deletes originals.

use crate::config::MassRenameDeleteConfig;
use crate::detectors::{Detector, DetectorState};
use crate::events::{DetectorResult, FileEvent, OperationType};

pub struct MassRenameDeleteDetector {
    count_threshold: u64,
    window_seconds: u64,
}

impl MassRenameDeleteDetector {
    pub fn new(config: &MassRenameDeleteConfig) -> Self {
        Self {
            count_threshold: config.count_threshold,
            window_seconds: config.window_seconds,
        }
    }

    fn count_recent(&self, state: &DetectorState, process_id: u32, current_ns: u64) -> u64 {
        let window_ns = self.window_seconds * 1_000_000_000;
        let cutoff = current_ns.saturating_sub(window_ns);

        state
            .get_timestamps(self.name(), process_id)
            .map(|ts| ts.iter().filter(|&&t| t >= cutoff).count() as u64)
            .unwrap_or(0)
    }
}

impl Detector for MassRenameDeleteDetector {
    fn name(&self) -> &str {
        "mass_rename_delete"
    }

    fn evaluate(&self, event: &FileEvent, state: &mut DetectorState) -> DetectorResult {
        if event.operation != OperationType::Rename && event.operation != OperationType::Delete {
            return DetectorResult::new(self.name(), 0.0, vec![], event.process_id);
        }

        state.add_timestamp(self.name(), event.process_id, event.timestamp_ns);

        let recent_count = self.count_recent(state, event.process_id, event.timestamp_ns);

        let score = if recent_count >= self.count_threshold {
            let ratio = recent_count as f64 / self.count_threshold as f64;
            (ratio * 0.5).min(1.0)
        } else if recent_count >= self.count_threshold / 2 {
            let ratio = recent_count as f64 / self.count_threshold as f64;
            (ratio * 0.3).min(0.5)
        } else {
            0.0
        };

        let evidence = if score > 0.0 {
            vec![
                format!(
                    "{} rename/delete ops in {}s (threshold: {})",
                    recent_count, self.window_seconds, self.count_threshold
                ),
                format!("Operation: {:?} on {}", event.operation, event.file_path),
            ]
        } else {
            vec![]
        };

        DetectorResult::new(self.name(), score, evidence, event.process_id)
    }

    fn reset_process_state(&self, state: &mut DetectorState, process_id: u32) {
        if let Some(map) = state.timestamps.get_mut(self.name()) {
            map.remove(&process_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignores_non_rename_delete() {
        let config = MassRenameDeleteConfig {
            count_threshold: 5,
            window_seconds: 10,
        };
        let detector = MassRenameDeleteDetector::new(&config);
        let mut state = DetectorState::new();

        let event = FileEvent::new(
            1, 100, "test.exe".into(),
            OperationType::Write, r"C:\test.txt".into(),
        );
        let result = detector.evaluate(&event, &mut state);
        assert!((result.score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_detects_mass_renames() {
        let config = MassRenameDeleteConfig {
            count_threshold: 5,
            window_seconds: 60,
        };
        let detector = MassRenameDeleteDetector::new(&config);
        let mut state = DetectorState::new();
        let base_ts = 1_000_000_000_000u64;

        for i in 0..10 {
            let mut event = FileEvent::new(
                i, 100, "test.exe".into(),
                OperationType::Rename,
                format!(r"C:\docs\file{}.txt", i),
            );
            event.timestamp_ns = base_ts + (i * 100_000_000);
            let _ = detector.evaluate(&event, &mut state);
        }

        let mut event = FileEvent::new(
            11, 100, "test.exe".into(),
            OperationType::Delete, r"C:\docs\final.txt".into(),
        );
        event.timestamp_ns = base_ts + (10 * 100_000_000);
        let result = detector.evaluate(&event, &mut state);
        assert!(result.score > 0.0);
    }
}
