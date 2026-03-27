//! Extension Explosion Detector
//!
//! Detects processes creating files with many previously-unseen
//! file extensions, indicative of ransomware appending custom
//! extensions to encrypted files.

use crate::config::ExtensionExplosionConfig;
use crate::detectors::{Detector, DetectorState};
use crate::events::{DetectorResult, FileEvent, OperationType};

pub struct ExtensionExplosionDetector {
    new_extension_threshold: usize,
    window_seconds: u64,
}

impl ExtensionExplosionDetector {
    pub fn new(config: &ExtensionExplosionConfig) -> Self {
        Self {
            new_extension_threshold: config.new_extension_threshold,
            window_seconds: config.window_seconds,
        }
    }
}

impl Detector for ExtensionExplosionDetector {
    fn name(&self) -> &str {
        "extension_explosion"
    }

    fn evaluate(&self, event: &FileEvent, state: &mut DetectorState) -> DetectorResult {
        // Only track creates and renames (new file extension creation)
        if !matches!(
            event.operation,
            OperationType::Create | OperationType::Rename | OperationType::Write
        ) {
            return DetectorResult::new(self.name(), 0.0, vec![], event.process_id);
        }

        // Determine the extension to check.
        // For renames, extract from the new path; otherwise use the pre-parsed extension.
        // We avoid cloning the extension string unless we actually need to insert it.
        let extension: String = if event.operation == OperationType::Rename {
            event
                .new_file_path
                .as_ref()
                .and_then(|p| p.rsplit('.').next())
                .unwrap_or("")
                .to_ascii_lowercase()
        } else {
            // file_extension is already lowercase from the driver/parser,
            // so we can borrow it directly with minimal cost.
            if event.file_extension.is_empty() || event.file_extension.len() > 20 {
                return DetectorResult::new(self.name(), 0.0, vec![], event.process_id);
            }
            event.file_extension.clone()
        };

        if extension.is_empty() || extension.len() > 20 {
            return DetectorResult::new(self.name(), 0.0, vec![], event.process_id);
        }

        // Track unique extensions per process
        let unique_count = state.add_to_set(self.name(), event.process_id, extension);

        // Track timestamps for windowing
        state.add_timestamp(self.name(), event.process_id, event.timestamp_ns);

        let score = if unique_count > self.new_extension_threshold {
            let ratio = unique_count as f64 / self.new_extension_threshold as f64;
            ((ratio - 1.0) * 0.5 + 0.3).min(1.0)
        } else {
            0.0
        };

        let evidence = if score > 0.0 {
            let exts = state
                .get_set(self.name(), event.process_id)
                .map(|s| s.join(", "))
                .unwrap_or_default();
            vec![
                format!(
                    "{} unique extensions (threshold: {})",
                    unique_count, self.new_extension_threshold
                ),
                format!("Extensions: {}", exts),
                format!("Process: {} (PID: {})", event.process_name, event.process_id),
            ]
        } else {
            vec![]
        };

        DetectorResult::new(self.name(), score, evidence, event.process_id)
    }

    fn reset_process_state(&self, state: &mut DetectorState, process_id: u32) {
        if let Some(map) = state.string_sets.get_mut(self.name()) {
            map.remove(&process_id);
        }
        if let Some(map) = state.timestamps.get_mut(self.name()) {
            map.remove(&process_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_extensions() {
        let config = ExtensionExplosionConfig {
            new_extension_threshold: 10,
            window_seconds: 30,
        };
        let detector = ExtensionExplosionDetector::new(&config);
        let mut state = DetectorState::new();

        let mut event = FileEvent::new(
            1, 100, "app.exe".into(),
            OperationType::Create, r"C:\test\output.txt".into(),
        );
        event.file_extension = "txt".into();
        let result = detector.evaluate(&event, &mut state);
        assert!((result.score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extension_explosion() {
        let config = ExtensionExplosionConfig {
            new_extension_threshold: 3,
            window_seconds: 30,
        };
        let detector = ExtensionExplosionDetector::new(&config);
        let mut state = DetectorState::new();

        let extensions = [
            "locked", "encrypted", "crypt", "enc",
            "aaa", "bbb", "ccc", "ddd",
        ];

        let mut last_result = DetectorResult::new("", 0.0, vec![], 0);
        for (i, ext) in extensions.iter().enumerate() {
            let mut event = FileEvent::new(
                i as u64, 100, "ransom.exe".into(),
                OperationType::Create,
                format!(r"C:\Users\victim\file{}.{}", i, ext),
            );
            event.file_extension = ext.to_string();
            last_result = detector.evaluate(&event, &mut state);
        }

        assert!(last_result.score > 0.0);
    }
}
