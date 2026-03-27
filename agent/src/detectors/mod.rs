//! SentinelGuard Detector Framework
//!
//! Defines the Detector trait and provides the detector registry.
//! Each detector evaluates file events and returns a score in [0.0, 1.0]
//! with evidence strings.

pub mod entropy;
pub mod extension_explosion;
pub mod mass_rename_delete;
pub mod mass_write;
pub mod process_behavior;
pub mod ransom_note;
pub mod shadow_copy;

use crate::config::DetectorsConfig;
use crate::events::{DetectorResult, FileEvent};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Trait that all detectors must implement.
/// Detectors are stateful per-process and evaluate events in the context
/// of their accumulated state within a time window.
pub trait Detector: Send + Sync {
    /// Unique name of this detector
    fn name(&self) -> &str;

    /// Evaluate a single event. Returns a DetectorResult with score and evidence.
    fn evaluate(&self, event: &FileEvent, state: &mut DetectorState) -> DetectorResult;

    /// Reset state for a process (called when time window expires)
    fn reset_process_state(&self, state: &mut DetectorState, process_id: u32);
}

/// Per-process state that detectors can read and write to.
/// Each detector stores its own state keyed by detector name.
#[derive(Debug, Default, Clone)]
pub struct DetectorState {
    /// Counts per detector per process
    pub counters: HashMap<String, HashMap<u32, u64>>,
    /// String sets per detector per process (for tracking unique values)
    pub string_sets: HashMap<String, HashMap<u32, Vec<String>>>,
    /// Timestamps per detector per process
    pub timestamps: HashMap<String, HashMap<u32, Vec<u64>>>,
    /// Float accumulators per detector per process
    pub accumulators: HashMap<String, HashMap<u32, f64>>,
}

impl DetectorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment a counter for a detector/process pair
    pub fn increment_counter(&mut self, detector: &str, process_id: u32) -> u64 {
        let counter = self
            .counters
            .entry(detector.to_string())
            .or_default()
            .entry(process_id)
            .or_insert(0);
        *counter += 1;
        *counter
    }

    /// Get current counter value
    pub fn get_counter(&self, detector: &str, process_id: u32) -> u64 {
        self.counters
            .get(detector)
            .and_then(|m| m.get(&process_id))
            .copied()
            .unwrap_or(0)
    }

    /// Add a string to a set (returns current count of unique strings).
    /// Capped at 5,000 entries per process to prevent unbounded memory growth.
    pub fn add_to_set(&mut self, detector: &str, process_id: u32, value: String) -> usize {
        let set = self
            .string_sets
            .entry(detector.to_string())
            .or_default()
            .entry(process_id)
            .or_default();
        if set.len() >= 5_000 {
            // Cap reached — don't add more, just return current count
            return set.len();
        }
        if !set.contains(&value) {
            set.push(value);
        }
        set.len()
    }

    /// Get unique string set
    pub fn get_set(&self, detector: &str, process_id: u32) -> Option<&Vec<String>> {
        self.string_sets
            .get(detector)
            .and_then(|m| m.get(&process_id))
    }

    /// Add a timestamp.
    /// Capped at 10,000 per process — when exceeded, the oldest half is drained.
    pub fn add_timestamp(&mut self, detector: &str, process_id: u32, ts: u64) {
        let timestamps = self
            .timestamps
            .entry(detector.to_string())
            .or_default()
            .entry(process_id)
            .or_default();
        timestamps.push(ts);
        // Prevent unbounded growth: trim oldest half when over 10k
        if timestamps.len() > 10_000 {
            let drain_to = timestamps.len() / 2;
            timestamps.drain(..drain_to);
        }
    }

    /// Get timestamps for a detector/process
    pub fn get_timestamps(&self, detector: &str, process_id: u32) -> Option<&Vec<u64>> {
        self.timestamps
            .get(detector)
            .and_then(|m| m.get(&process_id))
    }

    /// Clear all state for a specific process across all detectors
    pub fn clear_process(&mut self, process_id: u32) {
        for map in self.counters.values_mut() {
            map.remove(&process_id);
        }
        for map in self.string_sets.values_mut() {
            map.remove(&process_id);
        }
        for map in self.timestamps.values_mut() {
            map.remove(&process_id);
        }
        for map in self.accumulators.values_mut() {
            map.remove(&process_id);
        }
    }
}

/// Registry that holds all active detectors and manages evaluation
pub struct DetectorRegistry {
    detectors: Vec<Box<dyn Detector>>,
    state: Arc<RwLock<DetectorState>>,
}

impl DetectorRegistry {
    /// Create a new registry with all standard detectors configured
    pub fn new(config: &DetectorsConfig) -> Self {
        let detectors: Vec<Box<dyn Detector>> = vec![
            Box::new(entropy::EntropyDetector::new(&config.entropy)),
            Box::new(mass_write::MassWriteDetector::new(&config.mass_write)),
            Box::new(mass_rename_delete::MassRenameDeleteDetector::new(
                &config.mass_rename_delete,
            )),
            Box::new(ransom_note::RansomNoteDetector::new(&config.ransom_note)),
            Box::new(shadow_copy::ShadowCopyDetector::new(&config.shadow_copy)),
            Box::new(process_behavior::ProcessBehaviorDetector::new(
                &config.process_behavior,
            )),
            Box::new(extension_explosion::ExtensionExplosionDetector::new(
                &config.extension_explosion,
            )),
        ];

        Self {
            detectors,
            state: Arc::new(RwLock::new(DetectorState::new())),
        }
    }

    /// Evaluate an event against all detectors
    pub fn evaluate_all(&self, event: &FileEvent) -> Vec<DetectorResult> {
        let mut state = self.state.write();
        self.detectors
            .iter()
            .map(|d| d.evaluate(event, &mut state))
            .collect()
    }

    /// Reset state for a process across all detectors
    pub fn reset_process(&self, process_id: u32) {
        let mut state = self.state.write();
        state.clear_process(process_id);
    }

    /// Get detector names
    pub fn detector_names(&self) -> Vec<&str> {
        self.detectors.iter().map(|d| d.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_state_counters() {
        let mut state = DetectorState::new();
        assert_eq!(state.get_counter("test", 100), 0);
        assert_eq!(state.increment_counter("test", 100), 1);
        assert_eq!(state.increment_counter("test", 100), 2);
        assert_eq!(state.get_counter("test", 100), 2);
    }

    #[test]
    fn test_detector_state_sets() {
        let mut state = DetectorState::new();
        assert_eq!(state.add_to_set("test", 100, "a".to_string()), 1);
        assert_eq!(state.add_to_set("test", 100, "b".to_string()), 2);
        assert_eq!(state.add_to_set("test", 100, "a".to_string()), 2); // duplicate
    }

    #[test]
    fn test_clear_process() {
        let mut state = DetectorState::new();
        state.increment_counter("test", 100);
        state.add_to_set("test", 100, "a".to_string());
        state.clear_process(100);
        assert_eq!(state.get_counter("test", 100), 0);
        assert!(state.get_set("test", 100).is_none());
    }
}
