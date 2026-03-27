//! Process Behavior Detector
//!
//! Detects anomalous process behavior patterns such as high breadth
//! of file extensions accessed and excessive directory traversal.

use crate::config::ProcessBehaviorConfig;
use crate::detectors::{Detector, DetectorState};
use crate::events::{DetectorResult, FileEvent, OperationType};

pub struct ProcessBehaviorDetector {
    max_extensions: usize,
    max_directories: usize,
}

impl ProcessBehaviorDetector {
    pub fn new(config: &ProcessBehaviorConfig) -> Self {
        Self {
            max_extensions: config.max_extensions,
            max_directories: config.max_directories,
        }
    }
}

impl Detector for ProcessBehaviorDetector {
    fn name(&self) -> &str {
        "process_behavior"
    }

    fn evaluate(&self, event: &FileEvent, state: &mut DetectorState) -> DetectorResult {
        // Track for write, rename, and create operations
        if !matches!(
            event.operation,
            OperationType::Write | OperationType::Rename | OperationType::Create
        ) {
            return DetectorResult::new(self.name(), 0.0, vec![], event.process_id);
        }

        // Track unique extensions (static key avoids per-event format!() allocation)
        const EXT_KEY: &str = "process_behavior_extensions";
        if !event.file_extension.is_empty() {
            state.add_to_set(EXT_KEY, event.process_id, event.file_extension.clone());
        }

        // Track unique directories
        const DIR_KEY: &str = "process_behavior_directories";
        let dir = event.directory().to_string();
        if !dir.is_empty() {
            state.add_to_set(DIR_KEY, event.process_id, dir);
        }

        let ext_count = state
            .get_set(EXT_KEY, event.process_id)
            .map(|s| s.len())
            .unwrap_or(0);

        let dir_count = state
            .get_set(DIR_KEY, event.process_id)
            .map(|s| s.len())
            .unwrap_or(0);

        // Calculate score based on how far above thresholds we are
        let ext_score = if ext_count > self.max_extensions {
            let ratio = ext_count as f64 / self.max_extensions as f64;
            (ratio - 1.0).min(1.0) * 0.6
        } else {
            0.0
        };

        let dir_score = if dir_count > self.max_directories {
            let ratio = dir_count as f64 / self.max_directories as f64;
            (ratio - 1.0).min(1.0) * 0.4
        } else {
            0.0
        };

        let score = (ext_score + dir_score).min(1.0);

        let evidence = if score > 0.0 {
            vec![
                format!(
                    "Unique extensions: {} (max: {})",
                    ext_count, self.max_extensions
                ),
                format!(
                    "Unique directories: {} (max: {})",
                    dir_count, self.max_directories
                ),
                format!("Process: {} (PID: {})", event.process_name, event.process_id),
            ]
        } else {
            vec![]
        };

        DetectorResult::new(self.name(), score, evidence, event.process_id)
    }

    fn reset_process_state(&self, state: &mut DetectorState, process_id: u32) {
        const EXT_KEY: &str = "process_behavior_extensions";
        const DIR_KEY: &str = "process_behavior_directories";
        if let Some(map) = state.string_sets.get_mut(EXT_KEY) {
            map.remove(&process_id);
        }
        if let Some(map) = state.string_sets.get_mut(DIR_KEY) {
            map.remove(&process_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_behavior() {
        let config = ProcessBehaviorConfig {
            max_extensions: 15,
            max_directories: 50,
        };
        let detector = ProcessBehaviorDetector::new(&config);
        let mut state = DetectorState::new();

        let event = FileEvent::new(
            1, 100, "word.exe".into(),
            OperationType::Write, r"C:\Users\test\doc.docx".into(),
        );
        let result = detector.evaluate(&event, &mut state);
        assert!((result.score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_excessive_extensions() {
        let config = ProcessBehaviorConfig {
            max_extensions: 3,
            max_directories: 50,
        };
        let detector = ProcessBehaviorDetector::new(&config);
        let mut state = DetectorState::new();

        let extensions = ["txt", "doc", "pdf", "jpg", "png", "xlsx", "pptx"];
        for (i, ext) in extensions.iter().enumerate() {
            let mut event = FileEvent::new(
                i as u64, 100, "test.exe".into(),
                OperationType::Write,
                format!(r"C:\docs\file.{}", ext),
            );
            event.file_extension = ext.to_string();
            let _ = detector.evaluate(&event, &mut state);
        }

        let mut event = FileEvent::new(
            10, 100, "test.exe".into(),
            OperationType::Write, r"C:\docs\file.mp3".into(),
        );
        event.file_extension = "mp3".to_string();
        let result = detector.evaluate(&event, &mut state);
        assert!(result.score > 0.0);
    }
}
