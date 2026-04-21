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
    total_events: u64,
    ml_score: f64,
    final_score: f64,
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
            total_events: 0,
            ml_score: 0.0,
            final_score: 0.0,
        });

        // Evict old results outside the window
        pw.detector_results
            .retain(|r| r.timestamp_ns >= window_start);

        // Add new results and increment raw event count.
        // `results` contains one DetectorResult per active detector for the CURRENT file event.
        // Since evaluate_all returns N results for 1 file event, we just increment by 1.
        pw.detector_results.extend(results);
        pw.window_end_ns = now_ns;
        pw.window_start_ns = window_start;
        pw.total_events += 1;

        // Calculate weighted score using the latest result per detector
        let weighted_score = self.calculate_weighted_score(&pw.detector_results);

        // FIX: Decay stale ML scores when heuristic detectors have dropped
        // below the inference threshold. Without this, a transient spike
        // (e.g., browser touching many extensions briefly) would leave a high
        // ml_score cached forever, even after all detectors return to zero.
        const SCORE_DECAY_THRESHOLD: f64 = 0.1;
        if weighted_score < SCORE_DECAY_THRESHOLD && pw.ml_score > 0.0 {
            pw.ml_score = 0.0;
            pw.final_score = 0.0;
        }

        // Only include the highest-scoring result per detector for the UI.
        // This prevents the "Process Risk Details" from being a firehose of
        // thousands of mostly-zero results.
        let best_per_detector = Self::best_results_per_detector(&pw.detector_results);

        AggregatedScore {
            process_id,
            process_name: pw.process_name.clone(),
            weighted_score,
            ml_score: pw.ml_score,
            final_score: if pw.final_score > 0.0 { pw.final_score } else { weighted_score },
            detector_results: best_per_detector,
            window_start_ns: pw.window_start_ns,
            window_end_ns: pw.window_end_ns,
            total_events: pw.total_events,
        }
    }

    /// Calculate weighted score from detector results.
    /// Uses the maximum score seen for each detector in the current window.
    ///
    /// FIX: The old logic divided by the sum of "active" detector weights,
    /// which diluted high scores when other detectors returned 0.
    /// New logic: sum(score * weight) directly. Weights already sum to 1.0.
    /// A sensitivity floor ensures any single high-confidence detector
    /// pushes the score into alert territory.
    fn calculate_weighted_score(&self, results: &[DetectorResult]) -> f64 {
        // Get the maximum score for each detector in this window
        let mut latest_scores: HashMap<&str, f64> = HashMap::new();

        for result in results {
            let entry = latest_scores
                .entry(&result.detector_name)
                .or_insert(0.0);
            if result.score > *entry {
                *entry = result.score;
            }
        }

        let weight_map = [
            ("entropy_spike", self.weights.entropy_spike),
            ("mass_write", self.weights.mass_write),
            ("mass_rename_delete", self.weights.mass_rename_delete),
            ("ransom_note", self.weights.ransom_note),
            ("shadow_copy", self.weights.shadow_copy),
            ("process_behavior", self.weights.process_behavior),
            ("extension_explosion", self.weights.extension_explosion),
        ];

        // Sum score * weight directly
        let mut weighted_sum = 0.0;
        for (name, weight) in &weight_map {
            if let Some(&score) = latest_scores.get(name) {
                weighted_sum += score * weight;
            }
        }

        // Boost the sum to account for the fact that a real ransomware attack
        // typically only triggers 2-3 detectors perfectly (sum ~ 0.30 - 0.45).
        // A multiplier of 1.4 ensures multi-detector attacks reach quarantine
        // without over-amplifying single-detector benign noise (was 1.8).
        let boosted_sum = (weighted_sum * 1.4).min(1.0);

        // Sensitivity floor: if ANY single detector is very confident,
        // ensure the overall score reflects that (e.g. at least 0.25).
        // Reduced from 0.35 to avoid single-detector false-positive inflation.
        let max_single_score = latest_scores
            .values()
            .copied()
            .fold(0.0f64, f64::max);

        boosted_sum.max(max_single_score * 0.25).min(1.0)
    }

    /// Reduce a list of detector results to only the highest-scoring result
    /// per detector name. This keeps the UI clean.
    fn best_results_per_detector(results: &[DetectorResult]) -> Vec<DetectorResult> {
        let mut best: HashMap<&str, &DetectorResult> = HashMap::new();
        for r in results {
            let entry = best.entry(&r.detector_name).or_insert(r);
            if r.score > entry.score {
                *entry = r;
            }
        }
        best.into_values().filter(|r| r.score > 0.0).cloned().collect()
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
                let best_per_detector = Self::best_results_per_detector(&pw.detector_results);
                AggregatedScore {
                    process_id: pid,
                    process_name: pw.process_name.clone(),
                    weighted_score,
                    ml_score: pw.ml_score,
                    final_score: if pw.final_score > 0.0 { pw.final_score } else { weighted_score },
                    detector_results: best_per_detector,
                    window_start_ns: pw.window_start_ns,
                    window_end_ns: now_ns,
                    total_events: pw.total_events,
                }
            })
            .collect()
    }

    /// Remove a process from tracking
    pub fn remove_process(&self, process_id: u32) {
        self.windows.write().remove(&process_id);
    }

    /// Update the ML and final score for a process (called by inference pipeline)
    pub fn update_scores(&self, process_id: u32, ml_score: f64, final_score: f64) {
        if let Some(mut pw) = self.windows.write().get_mut(&process_id) {
            pw.ml_score = ml_score;
            pw.final_score = final_score;
        }
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
        // New scoring: weighted_sum = 0.16. Boosted (x1.4) = 0.224. Floor = 0.20. Max = 0.224.
        assert!((score.weighted_score - 0.224).abs() < 0.02);
    }

    #[test]
    fn test_sensitivity_floor_prevents_dilution() {
        // A single detector at max score should still produce a meaningful risk
        let correlator = Correlator::new(test_weights(), 60);
        let results = vec![DetectorResult::new("mass_write", 1.0, vec![], 100)];
        let score = correlator.add_results(100, "ransim.exe", results);
        // max(1.0 * 0.15 * 1.4, 1.0 * 0.25) = 0.25 — Medium risk, enough for ML inference
        assert!(score.weighted_score >= 0.21);
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
