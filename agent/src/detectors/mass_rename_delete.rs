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
        let time_window_start = event.timestamp - window_seconds;

        if stats.last_update < time_window_start {
            return 0.0;
        }

        let total_ops = stats.file_renames + stats.file_deletes;

        if total_ops >= self.config.rename_delete_threshold {
            let excess = total_ops - self.config.rename_delete_threshold;
            let score = (excess as f32 / self.config.rename_delete_threshold as f32).min(1.0);
            return score;
        }

        0.0
    }
}

