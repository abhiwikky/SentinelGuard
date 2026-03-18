//! Ransom Note Detector
//!
//! Detects creation of files matching known ransomware note patterns.

use crate::config::RansomNoteConfig;
use crate::detectors::{Detector, DetectorState};
use crate::events::{DetectorResult, FileEvent, OperationType};

pub struct RansomNoteDetector {
    patterns: Vec<String>,
}

impl RansomNoteDetector {
    pub fn new(config: &RansomNoteConfig) -> Self {
        Self {
            patterns: config
                .patterns
                .iter()
                .map(|p| p.to_lowercase())
                .collect(),
        }
    }

    fn matches_pattern(&self, filename: &str) -> Option<&str> {
        let lower = filename.to_lowercase();
        for pattern in &self.patterns {
            if lower.contains(pattern.as_str()) {
                return Some(pattern.as_str());
            }
        }
        None
    }
}

impl Detector for RansomNoteDetector {
    fn name(&self) -> &str {
        "ransom_note"
    }

    fn evaluate(&self, event: &FileEvent, state: &mut DetectorState) -> DetectorResult {
        // Only check create operations
        if event.operation != OperationType::Create {
            return DetectorResult::new(self.name(), 0.0, vec![], event.process_id);
        }

        let filename = event.filename();

        if let Some(matched_pattern) = self.matches_pattern(filename) {
            let count = state.increment_counter(self.name(), event.process_id);

            // First ransom note = 0.7, multiple = higher
            let score = (0.7 + (count as f64 - 1.0) * 0.1).min(1.0);

            let evidence = vec![
                format!("Ransom note pattern matched: '{}'", matched_pattern),
                format!("File: {}", event.file_path),
                format!("Total ransom notes from PID {}: {}", event.process_id, count),
            ];

            DetectorResult::new(self.name(), score, evidence, event.process_id)
        } else {
            DetectorResult::new(self.name(), 0.0, vec![], event.process_id)
        }
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
    fn test_detects_ransom_note() {
        let config = RansomNoteConfig {
            patterns: vec!["readme.txt".into(), "how_to_decrypt".into()],
        };
        let detector = RansomNoteDetector::new(&config);
        let mut state = DetectorState::new();

        let event = FileEvent::new(
            1, 100, "malware.exe".into(),
            OperationType::Create,
            r"C:\Users\victim\Documents\README.TXT".into(),
        );
        let result = detector.evaluate(&event, &mut state);
        assert!(result.score >= 0.7);
    }

    #[test]
    fn test_ignores_normal_files() {
        let config = RansomNoteConfig {
            patterns: vec!["readme.txt".into()],
        };
        let detector = RansomNoteDetector::new(&config);
        let mut state = DetectorState::new();

        let event = FileEvent::new(
            1, 100, "notepad.exe".into(),
            OperationType::Create,
            r"C:\Users\test\document.docx".into(),
        );
        let result = detector.evaluate(&event, &mut state);
        assert!((result.score - 0.0).abs() < f64::EPSILON);
    }
}
