//
// File Extension Explosion Detector
//

use anyhow::Result;
use std::collections::HashMap;
use crate::config::DetectorConfig;
use crate::detectors::{Detector, ProcessStats};
use crate::events::FileEvent;

pub struct FileExtensionDetector {
    config: DetectorConfig,
    extension_counts: HashMap<u32, HashMap<String, usize>>,
}

impl FileExtensionDetector {
    pub fn new(config: &DetectorConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            extension_counts: HashMap::new(),
        })
    }

    fn get_file_extension(path: &str) -> String {
        if let Some(pos) = path.rfind('.') {
            path[pos..].to_lowercase()
        } else {
            String::new()
        }
    }
}

impl Detector for FileExtensionDetector {
    fn name(&self) -> &str {
        "FileExtensionDetector"
    }

    fn analyze(&self, event: &FileEvent, _stats: &ProcessStats) -> f32 {
        if event.event_type != crate::events::EventType::FileCreate 
            && event.event_type != crate::events::EventType::FileWrite {
            return 0.0;
        }

        let extension = Self::get_file_extension(&event.file_path);
        
        // Known ransomware extensions
        let suspicious_extensions = [
            ".locked", ".encrypted", ".crypto", ".vault", 
            ".ecc", ".ezz", ".exx", ".zzz", ".aaa", ".micro",
            ".encrypted", ".crypted", ".vault", ".payfast",
        ];

        if suspicious_extensions.iter().any(|&ext| extension == ext) {
            return 1.0;
        }

        0.0
    }
}

