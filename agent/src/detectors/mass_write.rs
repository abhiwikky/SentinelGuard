//
// Mass Write Detector
//

use anyhow::Result;
use crate::config::DetectorConfig;
use crate::detectors::{Detector, ProcessStats};
use crate::events::FileEvent;

pub struct MassWriteDetector {
    config: DetectorConfig,
}

impl MassWriteDetector {
    pub fn new(config: &DetectorConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Detector for MassWriteDetector {
    fn name(&self) -> &str {
        "MassWriteDetector"
    }

    fn analyze(&self, event: &FileEvent, stats: &ProcessStats) -> f32 {
        if event.event_type != crate::events::EventType::FileWrite {
            return 0.0;
        }

        let window_seconds = self.config.mass_write_window_seconds as i64;
        let time_window_start = event.timestamp - window_seconds;

        // Check if writes are within the time window
        if stats.last_update < time_window_start {
            return 0.0;
        }

        // Calculate write rate
        let write_rate = stats.file_writes as f32 / self.config.mass_write_window_seconds as f32;

        if stats.file_writes >= self.config.mass_write_threshold {
            // Normalize score based on threshold
            let excess = stats.file_writes - self.config.mass_write_threshold;
            let score = (excess as f32 / self.config.mass_write_threshold as f32).min(1.0);
            return score;
        }

        0.0
    }
}

