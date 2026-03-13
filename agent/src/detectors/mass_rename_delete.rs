//
// Mass Rename/Delete Detector
//

use anyhow::Result;
use crate::config::DetectorConfig;
use crate::detectors::{Detector, ProcessStats};
use crate::events::FileEvent;

pub struct MassRenameDeleteDetector {
    config: DetectorConfig,
}

impl MassRenameDeleteDetector {
    pub fn new(config: &DetectorConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Detector for MassRenameDeleteDetector {
    fn name(&self) -> &str {
        "MassRenameDeleteDetector"
    }

    fn analyze(&self, event: &FileEvent, stats: &ProcessStats) -> f32 {
        let is_rename = event.event_type == crate::events::EventType::FileRename;
        let is_delete = event.event_type == crate::events::EventType::FileDelete;

        if !is_rename && !is_delete {
            return 0.0;
        }

        let window_seconds = self.config.rename_delete_window_seconds as i64;
        let total_ops = stats.rename_delete_count_since(event.timestamp, window_seconds);

        if total_ops >= self.config.rename_delete_threshold {
            let score = total_ops as f32 / self.config.rename_delete_threshold as f32;
            return score.min(1.0);
        }

        0.0
    }
}

