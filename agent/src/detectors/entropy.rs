//
// Entropy Spike Detector
//

use anyhow::Result;
use crate::config::DetectorConfig;
use crate::detectors::{calculate_entropy, Detector, ProcessStats};
use crate::events::FileEvent;

pub struct EntropyDetector {
    config: DetectorConfig,
}

impl EntropyDetector {
    pub fn new(config: &DetectorConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    fn calculate_shannon_entropy(&self, data: &[u8]) -> f32 {
        calculate_entropy(data)
    }
}

impl Detector for EntropyDetector {
    fn name(&self) -> &str {
        "EntropyDetector"
    }

    fn analyze(&self, event: &FileEvent, stats: &ProcessStats) -> f32 {
        if event.event_type != crate::events::EventType::FileWrite {
            return 0.0;
        }

        if event.entropy_preview.is_empty() {
            return 0.0;
        }

        let entropy = self.calculate_shannon_entropy(&event.entropy_preview);

        // Check for entropy spike
        let avg_entropy = stats.avg_entropy_before_event(event, self.config.mass_write_window_seconds as i64);
        if avg_entropy > 0.0 {
            let entropy_delta = entropy - avg_entropy;

            if entropy_delta > 0.3 && entropy > self.config.entropy_threshold {
                return 1.0;
            }
        }

        // High entropy alone is suspicious
        if entropy > self.config.entropy_threshold {
            return entropy;
        }

        0.0
    }
}

