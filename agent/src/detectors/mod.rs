//
// Detector modules
//

pub mod entropy;
pub mod file_extension;
pub mod mass_rename_delete;
pub mod mass_write;
pub mod process_behavior;
pub mod ransom_note;
pub mod shadow_copy;

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

use crate::config::Config;
use crate::events::{EventType, FileEvent};

const SHORT_WINDOW_SECONDS: i64 = 10;
const LONG_WINDOW_SECONDS: i64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorScores {
    pub process_id: u32,
    pub process_path: String,
    pub entropy_score: f32,
    pub mass_write_score: f32,
    pub mass_rename_delete_score: f32,
    pub ransom_note_score: f32,
    pub shadow_copy_score: f32,
    pub process_behavior_score: f32,
    pub file_extension_score: f32,
    pub event_rate: f32,
    pub avg_entropy_per_sec: f32,
    pub rename_delete_freq: f32,
    pub burst_interval: f32,
    pub num_detectors_firing: f32,
    pub file_diversity: f32,
    pub bytes_written_per_sec: f32,
    pub unique_extensions: f32,
    pub timestamp: i64,
    pub triggered_detectors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRiskSnapshot {
    pub process_id: u32,
    pub process_path: String,
    pub risk_score: f32,
    pub last_activity: i64,
    pub active_detectors: Vec<String>,
}

pub struct DetectorManager {
    process_stats: Arc<DashMap<u32, ProcessStats>>,
    latest_scores: Arc<DashMap<u32, DetectorScores>>,
    detectors: Vec<Box<dyn Detector + Send + Sync>>,
}

#[derive(Debug, Clone)]
struct TimedBytes {
    timestamp: i64,
    bytes: u64,
}

#[derive(Debug, Clone)]
struct TimedString {
    timestamp: i64,
    value: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ProcessStats {
    process_path: String,
    last_update: i64,
    recent_event_timestamps: VecDeque<i64>,
    write_events: VecDeque<TimedBytes>,
    rename_events: VecDeque<i64>,
    delete_events: VecDeque<i64>,
    entropy_samples: VecDeque<(i64, f32)>,
    file_paths: VecDeque<TimedString>,
    directories: VecDeque<TimedString>,
    extensions: VecDeque<TimedString>,
    last_burst_interval: f32,
}

pub(crate) trait Detector: Send + Sync {
    fn name(&self) -> &str;
    fn analyze(&self, event: &FileEvent, stats: &ProcessStats) -> f32;
}

impl DetectorManager {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        let detectors: Vec<Box<dyn Detector + Send + Sync>> = vec![
            Box::new(entropy::EntropyDetector::new(&config.detector_config)?),
            Box::new(mass_write::MassWriteDetector::new(&config.detector_config)?),
            Box::new(mass_rename_delete::MassRenameDeleteDetector::new(&config.detector_config)?),
            Box::new(ransom_note::RansomNoteDetector::new(&config.detector_config)?),
            Box::new(shadow_copy::ShadowCopyDetector::new(&config.detector_config)?),
            Box::new(process_behavior::ProcessBehaviorDetector::new(&config.detector_config)?),
            Box::new(file_extension::FileExtensionDetector::new(&config.detector_config)?),
        ];

        Ok(Self {
            process_stats: Arc::new(DashMap::new()),
            latest_scores: Arc::new(DashMap::new()),
            detectors,
        })
    }

    pub async fn process_events(
        &self,
        mut event_rx: mpsc::UnboundedReceiver<FileEvent>,
    ) -> Result<()> {
        while let Some(event) = event_rx.recv().await {
            if event.result != 0 {
                continue;
            }

            let scores = self.update_and_score(&event);
            self.latest_scores.insert(event.process_id, scores.clone());

            if !scores.triggered_detectors.is_empty() {
                debug!(
                    process_id = scores.process_id,
                    risk = overall_risk(&scores),
                    detectors = ?scores.triggered_detectors,
                    "detector outputs updated"
                );
            }
        }

        Ok(())
    }

    fn update_and_score(&self, event: &FileEvent) -> DetectorScores {
        let mut entry = self
            .process_stats
            .entry(event.process_id)
            .or_insert_with(|| ProcessStats::new(event));

        entry.record_event(event);
        self.build_scores(event, &entry)
    }

    fn build_scores(&self, event: &FileEvent, stats: &ProcessStats) -> DetectorScores {
        let mut scores = DetectorScores {
            process_id: event.process_id,
            process_path: stats.process_path.clone(),
            entropy_score: 0.0,
            mass_write_score: 0.0,
            mass_rename_delete_score: 0.0,
            ransom_note_score: 0.0,
            shadow_copy_score: 0.0,
            process_behavior_score: 0.0,
            file_extension_score: 0.0,
            event_rate: stats.event_rate(event.timestamp, SHORT_WINDOW_SECONDS),
            avg_entropy_per_sec: stats.avg_entropy_per_sec(event.timestamp, SHORT_WINDOW_SECONDS),
            rename_delete_freq: stats.rename_delete_freq(event.timestamp, SHORT_WINDOW_SECONDS),
            burst_interval: stats.last_burst_interval,
            num_detectors_firing: 0.0,
            file_diversity: stats.file_diversity(event.timestamp, SHORT_WINDOW_SECONDS) as f32,
            bytes_written_per_sec: stats.bytes_written_per_sec(event.timestamp, SHORT_WINDOW_SECONDS),
            unique_extensions: stats.unique_extensions(event.timestamp, SHORT_WINDOW_SECONDS) as f32,
            timestamp: event.timestamp,
            triggered_detectors: Vec::new(),
        };

        for detector in &self.detectors {
            let score = detector.analyze(event, stats).clamp(0.0, 1.0);
            match detector.name() {
                "EntropyDetector" => scores.entropy_score = score,
                "MassWriteDetector" => scores.mass_write_score = score,
                "MassRenameDeleteDetector" => scores.mass_rename_delete_score = score,
                "RansomNoteDetector" => scores.ransom_note_score = score,
                "ShadowCopyDetector" => scores.shadow_copy_score = score,
                "ProcessBehaviorDetector" => scores.process_behavior_score = score,
                "FileExtensionDetector" => scores.file_extension_score = score,
                _ => {}
            }

            if score > 0.0 {
                scores.triggered_detectors.push(detector.name().to_string());
            }
        }

        scores.num_detectors_firing = scores.triggered_detectors.len() as f32;
        scores
    }

    pub async fn get_aggregated_scores(&self) -> DetectorScores {
        self.latest_scores
            .iter()
            .map(|entry| entry.value().clone())
            .max_by(|left, right| overall_risk(left).partial_cmp(&overall_risk(right)).unwrap())
            .unwrap_or_else(empty_scores)
    }

    pub async fn get_process_risk_snapshots(&self) -> Vec<ProcessRiskSnapshot> {
        let mut snapshots: Vec<_> = self
            .latest_scores
            .iter()
            .map(|entry| {
                let scores = entry.value().clone();
                ProcessRiskSnapshot {
                    process_id: scores.process_id,
                    process_path: scores.process_path.clone(),
                    risk_score: overall_risk(&scores),
                    last_activity: scores.timestamp,
                    active_detectors: scores.triggered_detectors.clone(),
                }
            })
            .collect();

        snapshots.sort_by(|left, right| right.risk_score.partial_cmp(&left.risk_score).unwrap());
        snapshots
    }

    pub async fn all_scores(&self) -> Vec<DetectorScores> {
        self.latest_scores
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
}

impl ProcessStats {
    fn new(event: &FileEvent) -> Self {
        let mut stats = Self {
            process_path: event.process_path.clone(),
            last_update: event.timestamp,
            recent_event_timestamps: VecDeque::new(),
            write_events: VecDeque::new(),
            rename_events: VecDeque::new(),
            delete_events: VecDeque::new(),
            entropy_samples: VecDeque::new(),
            file_paths: VecDeque::new(),
            directories: VecDeque::new(),
            extensions: VecDeque::new(),
            last_burst_interval: SHORT_WINDOW_SECONDS as f32,
        };
        stats.record_event(event);
        stats
    }

    fn record_event(&mut self, event: &FileEvent) {
        if !event.process_path.is_empty() {
            self.process_path = event.process_path.clone();
        }

        let previous_update = self.last_update;
        self.last_update = event.timestamp;
        self.recent_event_timestamps.push_back(event.timestamp);

        if previous_update > 0 {
            let interval = (event.timestamp - previous_update).max(0) as f32;
            if interval > 0.0 {
                self.last_burst_interval = interval;
            }
        }

        self.file_paths.push_back(TimedString {
            timestamp: event.timestamp,
            value: event.file_path.clone(),
        });

        if let Some(directory) = Path::new(&event.file_path)
            .parent()
            .map(|path| path.to_string_lossy().to_string())
        {
            self.directories.push_back(TimedString {
                timestamp: event.timestamp,
                value: directory,
            });
        }

        let extension = file_extension_from_path(&event.file_path);
        if !extension.is_empty() {
            self.extensions.push_back(TimedString {
                timestamp: event.timestamp,
                value: extension,
            });
        }

        match event.event_type {
            EventType::FileWrite => {
                self.write_events.push_back(TimedBytes {
                    timestamp: event.timestamp,
                    bytes: event.bytes_written,
                });

                if !event.entropy_preview.is_empty() {
                    self.entropy_samples.push_back((
                        event.timestamp,
                        calculate_entropy(&event.entropy_preview),
                    ));
                }
            }
            EventType::FileRename => self.rename_events.push_back(event.timestamp),
            EventType::FileDelete => self.delete_events.push_back(event.timestamp),
            _ => {}
        }

        self.prune(event.timestamp);
    }

    fn prune(&mut self, now: i64) {
        prune_timestamps(&mut self.recent_event_timestamps, now, LONG_WINDOW_SECONDS);
        prune_timestamps(&mut self.rename_events, now, LONG_WINDOW_SECONDS);
        prune_timestamps(&mut self.delete_events, now, LONG_WINDOW_SECONDS);
        prune_timed_bytes(&mut self.write_events, now, LONG_WINDOW_SECONDS);
        prune_timed_f32(&mut self.entropy_samples, now, LONG_WINDOW_SECONDS);
        prune_timed_strings(&mut self.file_paths, now, LONG_WINDOW_SECONDS);
        prune_timed_strings(&mut self.directories, now, LONG_WINDOW_SECONDS);
        prune_timed_strings(&mut self.extensions, now, LONG_WINDOW_SECONDS);
    }

    pub(crate) fn write_count_since(&self, now: i64, window_seconds: i64) -> usize {
        self.write_events
            .iter()
            .filter(|entry| entry.timestamp >= now - window_seconds)
            .count()
    }

    pub(crate) fn rename_delete_count_since(&self, now: i64, window_seconds: i64) -> usize {
        self.rename_events
            .iter()
            .chain(self.delete_events.iter())
            .filter(|timestamp| **timestamp >= now - window_seconds)
            .count()
    }

    pub(crate) fn bytes_written_since(&self, now: i64, window_seconds: i64) -> u64 {
        self.write_events
            .iter()
            .filter(|entry| entry.timestamp >= now - window_seconds)
            .map(|entry| entry.bytes)
            .sum()
    }

    pub(crate) fn avg_entropy(&self, now: i64, window_seconds: i64) -> f32 {
        let mut total = 0.0;
        let mut count = 0.0;

        for (timestamp, entropy) in &self.entropy_samples {
            if *timestamp >= now - window_seconds {
                total += *entropy;
                count += 1.0;
            }
        }

        if count == 0.0 { 0.0 } else { total / count }
    }

    pub(crate) fn avg_entropy_before_event(&self, event: &FileEvent, window_seconds: i64) -> f32 {
        let current_entropy = if event.entropy_preview.is_empty() {
            return self.avg_entropy(event.timestamp, window_seconds);
        } else {
            calculate_entropy(&event.entropy_preview)
        };

        let mut total = 0.0;
        let mut count = 0.0;
        for (timestamp, entropy) in &self.entropy_samples {
            if *timestamp >= event.timestamp - window_seconds {
                total += *entropy;
                count += 1.0;
            }
        }

        if count <= 1.0 {
            0.0
        } else {
            ((total - current_entropy) / (count - 1.0)).max(0.0)
        }
    }

    pub(crate) fn event_rate(&self, now: i64, window_seconds: i64) -> f32 {
        self.recent_event_timestamps
            .iter()
            .filter(|timestamp| **timestamp >= now - window_seconds)
            .count() as f32
            / window_seconds as f32
    }

    pub(crate) fn avg_entropy_per_sec(&self, now: i64, window_seconds: i64) -> f32 {
        self.avg_entropy(now, window_seconds) * self.event_rate(now, window_seconds)
    }

    pub(crate) fn rename_delete_freq(&self, now: i64, window_seconds: i64) -> f32 {
        self.rename_delete_count_since(now, window_seconds) as f32 / window_seconds as f32
    }

    pub(crate) fn bytes_written_per_sec(&self, now: i64, window_seconds: i64) -> f32 {
        self.bytes_written_since(now, window_seconds) as f32 / window_seconds as f32
    }

    pub(crate) fn file_diversity(&self, now: i64, window_seconds: i64) -> usize {
        unique_count(&self.file_paths, now, window_seconds)
    }

    pub(crate) fn directory_diversity(&self, now: i64, window_seconds: i64) -> usize {
        unique_count(&self.directories, now, window_seconds)
    }

    pub(crate) fn unique_extensions(&self, now: i64, window_seconds: i64) -> usize {
        unique_count(&self.extensions, now, window_seconds)
    }
}

fn overall_risk(scores: &DetectorScores) -> f32 {
    let detector_scores = [
        scores.entropy_score,
        scores.mass_write_score,
        scores.mass_rename_delete_score,
        scores.ransom_note_score,
        scores.shadow_copy_score,
        scores.process_behavior_score,
        scores.file_extension_score,
    ];

    let active: Vec<f32> = detector_scores
        .into_iter()
        .filter(|score| *score > 0.0)
        .collect();

    if active.is_empty() {
        0.0
    } else {
        active.iter().sum::<f32>() / active.len() as f32
    }
}

fn empty_scores() -> DetectorScores {
    DetectorScores {
        process_id: 0,
        process_path: String::new(),
        entropy_score: 0.0,
        mass_write_score: 0.0,
        mass_rename_delete_score: 0.0,
        ransom_note_score: 0.0,
        shadow_copy_score: 0.0,
        process_behavior_score: 0.0,
        file_extension_score: 0.0,
        event_rate: 0.0,
        avg_entropy_per_sec: 0.0,
        rename_delete_freq: 0.0,
        burst_interval: SHORT_WINDOW_SECONDS as f32,
        num_detectors_firing: 0.0,
        file_diversity: 0.0,
        bytes_written_per_sec: 0.0,
        unique_extensions: 0.0,
        timestamp: 0,
        triggered_detectors: Vec::new(),
    }
}

fn unique_count(entries: &VecDeque<TimedString>, now: i64, window_seconds: i64) -> usize {
    entries
        .iter()
        .filter(|entry| entry.timestamp >= now - window_seconds)
        .map(|entry| entry.value.clone())
        .collect::<HashSet<_>>()
        .len()
}

fn prune_timestamps(entries: &mut VecDeque<i64>, now: i64, window_seconds: i64) {
    while matches!(entries.front(), Some(timestamp) if *timestamp < now - window_seconds) {
        entries.pop_front();
    }
}

fn prune_timed_bytes(entries: &mut VecDeque<TimedBytes>, now: i64, window_seconds: i64) {
    while matches!(entries.front(), Some(entry) if entry.timestamp < now - window_seconds) {
        entries.pop_front();
    }
}

fn prune_timed_f32(entries: &mut VecDeque<(i64, f32)>, now: i64, window_seconds: i64) {
    while matches!(entries.front(), Some((timestamp, _)) if *timestamp < now - window_seconds) {
        entries.pop_front();
    }
}

fn prune_timed_strings(entries: &mut VecDeque<TimedString>, now: i64, window_seconds: i64) {
    while matches!(entries.front(), Some(entry) if entry.timestamp < now - window_seconds) {
        entries.pop_front();
    }
}

fn file_extension_from_path(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
}

pub(crate) fn calculate_entropy(data: &[u8]) -> f32 {
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

    entropy / 8.0
}
