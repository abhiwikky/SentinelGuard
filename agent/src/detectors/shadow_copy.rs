//
// Shadow Copy Deletion Detector
//

use anyhow::Result;
use crate::config::DetectorConfig;
use crate::detectors::{Detector, ProcessStats};
use crate::events::FileEvent;

pub struct ShadowCopyDetector {
    config: DetectorConfig,
}

impl ShadowCopyDetector {
    pub fn new(config: &DetectorConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Detector for ShadowCopyDetector {
    fn name(&self) -> &str {
        "ShadowCopyDetector"
    }

    fn analyze(&self, event: &FileEvent, _stats: &ProcessStats) -> f32 {
        if event.event_type == crate::events::EventType::VSSDelete {
            return 1.0;
        }

        // Check for shadow copy deletion commands in process path
        let process_lower = event.process_path.to_lowercase();
        if process_lower.contains("vssadmin") 
            || process_lower.contains("wmic")
            || process_lower.contains("shadowcopy") {
            return 1.0;
        }

        0.0
    }
}

