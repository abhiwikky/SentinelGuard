//
// Ransom Note Detector
//

use anyhow::Result;
use crate::config::DetectorConfig;
use crate::detectors::{Detector, ProcessStats};
use crate::events::FileEvent;

pub struct RansomNoteDetector {
    config: DetectorConfig,
}

impl RansomNoteDetector {
    pub fn new(config: &DetectorConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    fn check_patterns(&self, text: &str) -> bool {
        let text_upper = text.to_uppercase();
        for pattern in &self.config.ransom_note_patterns {
            if text_upper.contains(&pattern.to_uppercase()) {
                return true;
            }
        }
        false
    }
}

impl Detector for RansomNoteDetector {
    fn name(&self) -> &str {
        "RansomNoteDetector"
    }

    fn analyze(&self, event: &FileEvent, _stats: &ProcessStats) -> f32 {
        // Check file path for ransom note patterns
        if self.check_patterns(&event.file_path) {
            return 1.0;
        }

        // Check process path
        if self.check_patterns(&event.process_path) {
            return 0.8;
        }

        // In production, would also scan file contents using YARA
        0.0
    }
}

