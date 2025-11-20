//
// Entropy Spike Detector
//

use anyhow::Result;
use crate::config::DetectorConfig;
use crate::detectors::{Detector, ProcessStats};
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
        if data.is_empty() {
            return 0.0;
        }

        let mut frequency = [0u32; 256];
        for &byte in data {
            frequency[byte as usize] += 1;
        }

        let len = data.len() as f32;
        let mut entropy = 0.0;

        for &count in &frequency {
            if count > 0 {
                let probability = count as f32 / len;
                entropy -= probability * probability.log2();
            }
        }

        entropy / 8.0  // Normalize to 0-1 range
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
        if stats.entropy_samples.len() >= 2 {
            let avg_entropy: f32 = stats.entropy_samples.iter().sum::<f32>() / stats.entropy_samples.len() as f32;
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

