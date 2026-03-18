//! SentinelGuard Correlation Module
//!
//! Aggregates detector results per process within sliding time windows
//! and produces final risk scores using weighted averaging and ML inference.

use crate::config::DetectorWeights;
use crate::events::{AggregatedScore, DetectorResult};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Per-process correlation state
#[derive(Debug, Clone)]
struct ProcessWindow {
    process_name: String,
    detector_results: Vec<DetectorResult>,
    window_start_ns: u64,
    window_end_ns: u64,
}

/// Correlator aggregates detector outputs per process within time windows.
pub struct Correlator {
    weights: DetectorWeights,
    window_seconds: u64,
    /// Per-process windows: process_id -> ProcessWindow
    windows: Arc<RwLock<HashMap<u32, ProcessWindow>>>,
}

impl Correlator {
    pub fn new(weights: DetectorWeights, window_seconds: u64) -> Self {
        Self {
            weights,
            window_seconds,
            windows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add detector results for a process and get the current aggregated score.
    pub fn add_results(
        &self,
        process_id: u32,
        process_name: &str,
        results: Vec<DetectorResult>,
    ) -> AggregatedScore {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let window_ns = self.window_seconds * 1_000_000_000;
        let window_start = now_ns.saturating_sub(window_ns);

        let mut windows = self.windows.write();

        let pw = windows.entry(process_id).or_insert_with(|| ProcessWindow {
            process_name: process_name.to_string(),
            detector_results: Vec::new(),
            window_start_ns: window_start,
            window_end_ns: now_ns,
        });

        // Evict old results outside the window
        pw.detector_results
            .retain(|r| r.timestamp_ns >= window_start);

        // Add new results
        pw.detector_results.extend(results);
        pw.window_end_ns = now_ns;
        pw.window_start_ns = window_start;

        // Calculate weighted score using the latest result per detector
        let weighted_score = self.calculate_weighted_score(&pw.detector_results);

        AggregatedScore {
            process_id,
            process_name: pw.process_name.clone(),
            weighted_score,
            ml_score: 0.0, // Set by inference module
            final_score: weighted_score, // Updated after ML
            detector_results: pw.detector_results.clone(),
            window_start_ns: pw.window_start_ns,
            window_end_ns: pw.window_end_ns,
        }
    }

    /// Calculate weighted average score from detector results.
    /// Uses the latest result from each detector.
    fn calculate_weighted_score(&self, results: &[DetectorResult]) -> f64 {
        // Get the latest score for each detector
        let mut latest_scores: HashMap<&str, f64> = HashMap::new();

        for result in results {
            let entry = latest_scores
                .entry(&result.detector_name)
                .or_insert(0.0);
            // Keep the maximum score seen in this window
            if result.score > *entry {
                *entry = result.score;
            }
        }

        // Apply weights
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        let weight_map = [
            ("entropy_spike", self.weights.entropy_spike),
            ("mass_write", self.weights.mass_write),
            ("mass_rename_delete", self.weights.mass_rename_delete),
            ("ransom_note", self.weights.ransom_note),
            ("shadow_copy", self.weights.shadow_copy),
            ("process_behavior", self.weights.process_behavior),
            ("extension_explosion", self.weights.extension_explosion),
        ];

        for (name, weight) in &weight_map {
            if let Some(&score) = latest_scores.get(name) {
                weighted_sum += score * weight;
                weight_sum += weight;
            }
        }

        if weight_sum > 0.0 {
            (weighted_sum / weight_sum).min(1.0)
        } else {
            0.0
        }
    }

    /// Get current risk scores for all tracked processes
    pub fn get_all_scores(&self) -> Vec<AggregatedScore> {
        let windows = self.windows.read();
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        windows
            .iter()
            .map(|(&pid, pw)| {
                let weighted_score = self.calculate_weighted_score(&pw.detector_results);
                AggregatedScore {
                    process_id: pid,
                    process_name: pw.process_name.clone(),
                    weighted_score,
                    ml_score: 0.0,
                    final_score: weighted_score,
                    detector_results: pw.detector_results.clone(),
                    window_start_ns: pw.window_start_ns,
                    window_end_ns: now_ns,
                }
            })
            .collect()
    }

    /// Remove a process from tracking
    pub fn remove_process(&self, process_id: u32) {
        self.windows.write().remove(&process_id);
    }

    /// Clean up expired windows
    pub fn cleanup_expired(&self) {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let window_ns = self.window_seconds * 1_000_000_000;
        let cutoff = now_ns.saturating_sub(window_ns * 2); // Keep for 2x window

        let mut windows = self.windows.write();
        windows.retain(|_, pw| pw.window_end_ns >= cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DetectorWeights;

    fn test_weights() -> DetectorWeights {
        DetectorWeights {
            entropy_spike: 0.20,
            mass_write: 0.15,
            mass_rename_delete: 0.15,
            ransom_note: 0.15,
            shadow_copy: 0.10,
            process_behavior: 0.15,
            extension_explosion: 0.10,
        }
    }

    #[test]
    fn test_empty_results() {
        let correlator = Correlator::new(test_weights(), 60);
        let score = correlator.add_results(100, "test.exe", vec![]);
        assert!((score.weighted_score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_single_detector() {
        let correlator = Correlator::new(test_weights(), 60);
        let results = vec![DetectorResult::new("entropy_spike", 0.8, vec![], 100)];
        let score = correlator.add_results(100, "test.exe", results);
        assert!((score.weighted_score - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_multiple_detectors() {
        let correlator = Correlator::new(test_weights(), 60);
        let results = vec![
            DetectorResult::new("entropy_spike", 0.9, vec![], 100),
            DetectorResult::new("mass_write", 0.7, vec![], 100),
            DetectorResult::new("ransom_note", 0.8, vec![], 100),
        ];
        let score = correlator.add_results(100, "test.exe", results);
        assert!(score.weighted_score > 0.0);
        assert!(score.weighted_score <= 1.0);
    }

    #[test]
    fn test_process_tracking() {
        let correlator = Correlator::new(test_weights(), 60);
        let _ = correlator.add_results(
            100,
            "a.exe",
            vec![DetectorResult::new("entropy_spike", 0.5, vec![], 100)],
        );
        let _ = correlator.add_results(
            200,
            "b.exe",
            vec![DetectorResult::new("mass_write", 0.3, vec![], 200)],
        );

        let all = correlator.get_all_scores();
        assert_eq!(all.len(), 2);
    }
}
