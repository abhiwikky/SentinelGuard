//
// Process Behavior Detector
//

use anyhow::Result;
use crate::config::DetectorConfig;
use crate::detectors::{Detector, ProcessStats};
use crate::events::FileEvent;

pub struct ProcessBehaviorDetector {
    config: DetectorConfig,
}

impl ProcessBehaviorDetector {
    pub fn new(config: &DetectorConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Detector for ProcessBehaviorDetector {
    fn name(&self) -> &str {
        "ProcessBehaviorDetector"
    }

    fn analyze(&self, event: &FileEvent, _stats: &ProcessStats) -> f32 {
        // Check for suspicious process paths
        let process_lower = event.process_path.to_lowercase();
        
        // Suspicious locations
        if process_lower.contains("temp") 
            || process_lower.contains("appdata\\local\\temp")
            || process_lower.contains("downloads") {
            return 0.3;
        }

        // Suspicious file extensions in process path
        if process_lower.ends_with(".exe") {
            // Check if executable is in unusual location
            if !process_lower.contains("program files") 
                && !process_lower.contains("windows\\system32") {
                return 0.2;
            }
        }

        0.0
    }
}

