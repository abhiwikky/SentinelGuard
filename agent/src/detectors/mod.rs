//
// Detector modules
//

pub mod entropy;
pub mod mass_write;
pub mod mass_rename_delete;
pub mod ransom_note;
pub mod shadow_copy;
pub mod process_behavior;
pub mod file_extension;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::events::FileEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorScores {
    pub process_id: u32,
    pub entropy_score: f32,
    pub mass_write_score: f32,
    pub mass_rename_delete_score: f32,
    pub ransom_note_score: f32,
    pub shadow_copy_score: f32,
    pub process_behavior_score: f32,
    pub file_extension_score: f32,
    pub timestamp: i64,
}

pub struct DetectorManager {
    config: Arc<Config>,
    process_stats: Arc<DashMap<u32, ProcessStats>>,
    detectors: Vec<Box<dyn Detector + Send + Sync>>,
}

struct ProcessStats {
    file_writes: usize,
    file_renames: usize,
    file_deletes: usize,
    total_bytes_written: u64,
    entropy_samples: Vec<f32>,
    last_update: i64,
}

pub trait Detector: Send + Sync {
    fn name(&self) -> &str;
    fn analyze(&self, event: &FileEvent, stats: &ProcessStats) -> f32;
}

impl DetectorManager {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        let mut detectors: Vec<Box<dyn Detector + Send + Sync>> = Vec::new();
        
        detectors.push(Box::new(entropy::EntropyDetector::new(&config.detector_config)?));
        detectors.push(Box::new(mass_write::MassWriteDetector::new(&config.detector_config)?));
        detectors.push(Box::new(mass_rename_delete::MassRenameDeleteDetector::new(&config.detector_config)?));
        detectors.push(Box::new(ransom_note::RansomNoteDetector::new(&config.detector_config)?));
        detectors.push(Box::new(shadow_copy::ShadowCopyDetector::new(&config.detector_config)?));
        detectors.push(Box::new(process_behavior::ProcessBehaviorDetector::new(&config.detector_config)?));
        detectors.push(Box::new(file_extension::FileExtensionDetector::new(&config.detector_config)?));

        Ok(Self {
            config,
            process_stats: Arc::new(DashMap::new()),
            detectors,
        })
    }

    pub async fn process_events(self, mut event_rx: mpsc::UnboundedReceiver<FileEvent>) -> Result<()> {
        while let Some(event) = event_rx.recv().await {
            self.update_process_stats(&event);
            
            // Run all detectors
            if let Some(stats) = self.process_stats.get(&event.process_id) {
                for detector in &self.detectors {
                    let _score = detector.analyze(&event, &stats);
                }
            }
        }

        Ok(())
    }

    fn update_process_stats(&self, event: &FileEvent) {
        let mut entry = self.process_stats
            .entry(event.process_id)
            .or_insert_with(|| ProcessStats {
                file_writes: 0,
                file_renames: 0,
                file_deletes: 0,
                total_bytes_written: 0,
                entropy_samples: Vec::new(),
                last_update: event.timestamp,
            });

        match event.event_type {
            crate::events::EventType::FileWrite => {
                entry.file_writes += 1;
                entry.total_bytes_written += event.bytes_written;
            }
            crate::events::EventType::FileRename => {
                entry.file_renames += 1;
            }
            crate::events::EventType::FileDelete => {
                entry.file_deletes += 1;
            }
            _ => {}
        }

        entry.last_update = event.timestamp;
    }

    pub async fn get_aggregated_scores(&self) -> DetectorScores {
        // Aggregate scores from all processes
        // For now, return scores for the highest-risk process
        let mut max_score = 0.0;
        let mut max_process_id = 0;

        for entry in self.process_stats.iter() {
            let process_id = *entry.key();
            let stats = entry.value();
            
            // Calculate aggregate score (simplified)
            let score = (stats.file_writes as f32 * 0.1) 
                + (stats.file_renames as f32 * 0.2)
                + (stats.file_deletes as f32 * 0.2);

            if score > max_score {
                max_score = score;
                max_process_id = process_id;
            }
        }

        DetectorScores {
            process_id: max_process_id,
            entropy_score: 0.0,
            mass_write_score: 0.0,
            mass_rename_delete_score: 0.0,
            ransom_note_score: 0.0,
            shadow_copy_score: 0.0,
            process_behavior_score: 0.0,
            file_extension_score: 0.0,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

