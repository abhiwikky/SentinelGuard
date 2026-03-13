//
// Shadow Copy Deletion Detector
//

use anyhow::Result;
use crate::config::DetectorConfig;
use crate::detectors::{Detector, ProcessStats};
use crate::events::FileEvent;

pub struct ShadowCopyDetector {
    _config: DetectorConfig,
}

impl ShadowCopyDetector {
    pub fn new(config: &DetectorConfig) -> Result<Self> {
        Ok(Self {
            _config: config.clone(),
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

        let process_lower = event.process_path.to_lowercase();
        let file_lower = event.file_path.to_lowercase();
        if process_lower.contains("vssadmin")
            || process_lower.contains("wmic")
            || process_lower.contains("shadowcopy")
            || file_lower.contains("system volume information") {
            return 1.0;
        }

        0.0
    }
}

