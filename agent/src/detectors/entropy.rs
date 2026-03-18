//! Entropy Spike Detector
//!
//! Detects suspiciously high Shannon entropy in written files,
//! which is a strong indicator of encryption (ransomware).

use crate::config::EntropyConfig;
use crate::detectors::{Detector, DetectorState};
use crate::events::{DetectorResult, FileEvent, OperationType};

pub struct EntropyDetector {
    threshold: f64,
    min_file_size: u64,
}

impl EntropyDetector {
    pub fn new(config: &EntropyConfig) -> Self {
        Self {
            threshold: config.threshold,
            min_file_size: config.min_file_size,
        }
    }

    /// Calculate Shannon entropy from a byte distribution.
    /// Returns a value between 0.0 (uniform) and 8.0 (maximum entropy for bytes).
    pub fn calculate_entropy(data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut frequency = [0u64; 256];
        for &byte in data {
            frequency[byte as usize] += 1;
        }

        let len = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &frequency {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        entropy
    }
}

impl Detector for EntropyDetector {
    fn name(&self) -> &str {
        "entropy_spike"
    }

    fn evaluate(&self, event: &FileEvent, state: &mut DetectorState) -> DetectorResult {
        // Only evaluate write operations with sufficient file size
        if event.operation != OperationType::Write || event.file_size < self.min_file_size {
            return DetectorResult::new(self.name(), 0.0, vec![], event.process_id);
        }

        // Use the entropy value from the event if already computed,
        // otherwise we rely on the fact that entropy is set by the
        // ingestion pipeline when file content is available.
        let entropy = event.entropy;

        // Track the highest entropy per process
        let key = format!("{}_max_entropy", self.name());
        let current_max = state
            .accumulators
            .entry(key.clone())
            .or_default()
            .entry(event.process_id)
            .or_insert(0.0);

        if entropy > *current_max {
            *current_max = entropy;
        }

        // Count high-entropy writes
        let count = if entropy > self.threshold {
            state.increment_counter(self.name(), event.process_id)
        } else {
            state.get_counter(self.name(), event.process_id)
        };

        // Score based on how many high-entropy writes we've seen
        let score = if entropy > self.threshold {
            let base = (entropy - self.threshold) / (8.0 - self.threshold);
            let repetition_factor = (count as f64 / 5.0).min(1.0);
            (base * 0.6 + repetition_factor * 0.4).min(1.0)
        } else {
            0.0
        };

        let evidence = if score > 0.0 {
            vec![
                format!("Entropy: {:.2} (threshold: {:.1})", entropy, self.threshold),
                format!("High-entropy writes by PID {}: {}", event.process_id, count),
                format!("File: {}", event.file_path),
            ]
        } else {
            vec![]
        };

        DetectorResult::new(self.name(), score, evidence, event.process_id)
    }

    fn reset_process_state(&self, state: &mut DetectorState, process_id: u32) {
        if let Some(map) = state.counters.get_mut(self.name()) {
            map.remove(&process_id);
        }
        let key = format!("{}_max_entropy", self.name());
        if let Some(map) = state.accumulators.get_mut(&key) {
            map.remove(&process_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation() {
        // All zeros = 0 entropy
        let zeros = vec![0u8; 1000];
        assert!((EntropyDetector::calculate_entropy(&zeros) - 0.0).abs() < 0.001);

        // Random-like data should have high entropy
        let random: Vec<u8> = (0..=255).cycle().take(1024).collect();
        let ent = EntropyDetector::calculate_entropy(&random);
        assert!(ent > 7.9); // Near maximum for 256 unique values
    }

    #[test]
    fn test_entropy_detector_low_entropy() {
        let config = EntropyConfig {
            threshold: 7.0,
            min_file_size: 1024,
        };
        let detector = EntropyDetector::new(&config);
        let mut state = DetectorState::new();

        let event = FileEvent::new(
            1, 100, "test.exe".into(),
            OperationType::Write, r"C:\test.txt".into(),
        );
        // entropy defaults to 0.0

        let result = detector.evaluate(&event, &mut state);
        assert!((result.score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_entropy_detector_high_entropy() {
        let config = EntropyConfig {
            threshold: 7.0,
            min_file_size: 100,
        };
        let detector = EntropyDetector::new(&config);
        let mut state = DetectorState::new();

        let mut event = FileEvent::new(
            1, 100, "test.exe".into(),
            OperationType::Write, r"C:\test.encrypted".into(),
        );
        event.file_size = 10000;
        event.entropy = 7.95;

        let result = detector.evaluate(&event, &mut state);
        assert!(result.score > 0.0);
        assert!(!result.evidence.is_empty());
    }
}
