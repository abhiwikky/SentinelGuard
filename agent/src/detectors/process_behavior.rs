//
// Process Behavior Detector
//

use anyhow::Result;
use crate::config::DetectorConfig;
use crate::detectors::{Detector, ProcessStats};
use crate::events::FileEvent;

pub struct ProcessBehaviorDetector {
    _config: DetectorConfig,
}

impl ProcessBehaviorDetector {
    pub fn new(config: &DetectorConfig) -> Result<Self> {
        Ok(Self {
            _config: config.clone(),
        })
    }
}

impl Detector for ProcessBehaviorDetector {
    fn name(&self) -> &str {
        "ProcessBehaviorDetector"
    }

    fn analyze(&self, event: &FileEvent, stats: &ProcessStats) -> f32 {
        let process_lower = event.process_path.to_lowercase();
        let mut score: f32 = 0.0;

        if process_lower.contains("temp")
            || process_lower.contains("appdata\\local\\temp")
            || process_lower.contains("downloads")
        {
            score = score.max(0.35);
        }

        if process_lower.ends_with(".exe")
            && !process_lower.contains("program files")
            && !process_lower.contains("windows\\system32")
        {
            score = score.max(0.25);
        }

        if stats.bytes_written_per_sec(event.timestamp, 10) > 50_000.0 {
            score = score.max(0.7);
        }

        if stats.directory_diversity(event.timestamp, 10) >= 5 {
            score = score.max(0.6);
        }

        score.clamp(0.0, 1.0)
    }
}

