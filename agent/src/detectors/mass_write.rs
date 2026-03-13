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
        let window_seconds = self.config.mass_write_window_seconds as i64;
        let writes_in_window = stats.write_count_since(event.timestamp, window_seconds);

        if writes_in_window >= self.config.mass_write_threshold {
            let score = writes_in_window as f32 / self.config.mass_write_threshold as f32;
            return score.min(1.0);
        }

        0.0
    }
}

