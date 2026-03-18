//! Mass Write Detector
//!
//! Detects processes performing an unusually high number of write
//! operations within a short time window.

use crate::config::MassWriteConfig;
use crate::detectors::{Detector, DetectorState};
use crate::events::{DetectorResult, FileEvent, OperationType};

pub struct MassWriteDetector {
    count_threshold: u64,
    window_seconds: u64,
}

impl MassWriteDetector {
    pub fn new(config: &MassWriteConfig) -> Self {
        Self {
            count_threshold: config.count_threshold,
            window_seconds: config.window_seconds,
        }
    }

    /// Count events within the time window
    fn count_recent(&self, state: &DetectorState, process_id: u32, current_ns: u64) -> u64 {
        let window_ns = self.window_seconds * 1_000_000_000;
        let cutoff = current_ns.saturating_sub(window_ns);

        state
            .get_timestamps(self.name(), process_id)
            .map(|ts| ts.iter().filter(|&&t| t >= cutoff).count() as u64)
            .unwrap_or(0)
    }
}

impl Detector for MassWriteDetector {
    fn name(&self) -> &str {
        "mass_write"
    }

    fn evaluate(&self, event: &FileEvent, state: &mut DetectorState) -> DetectorResult {
        if event.operation != OperationType::Write {
            return DetectorResult::new(self.name(), 0.0, vec![], event.process_id);
        }

        // Record this write timestamp
        state.add_timestamp(self.name(), event.process_id, event.timestamp_ns);

        // Count writes within window
        let recent_count = self.count_recent(state, event.process_id, event.timestamp_ns);

        // Score: ramp up from 0 at threshold/2 to 1.0 at 2x threshold
        let score = if recent_count >= self.count_threshold / 2 {
            let ratio = recent_count as f64 / self.count_threshold as f64;
            (ratio - 0.5).max(0.0).min(1.0)
        } else {
            0.0
        };

        let evidence = if score > 0.0 {
            vec![
                format!(
                    "{} writes in {}s window (threshold: {})",
                    recent_count, self.window_seconds, self.count_threshold
                ),
                format!("Process: {} (PID: {})", event.process_name, event.process_id),
            ]
        } else {
            vec![]
        };

        DetectorResult::new(self.name(), score, evidence, event.process_id)
    }

    fn reset_process_state(&self, state: &mut DetectorState, process_id: u32) {
        if let Some(map) = state.timestamps.get_mut(self.name()) {
            map.remove(&process_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_below_threshold() {
        let config = MassWriteConfig {
            count_threshold: 50,
            window_seconds: 10,
        };
        let detector = MassWriteDetector::new(&config);
        let mut state = DetectorState::new();

        let event = FileEvent::new(
            1, 100, "test.exe".into(),
            OperationType::Write, r"C:\test.txt".into(),
        );

        let result = detector.evaluate(&event, &mut state);
        assert!((result.score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_above_threshold() {
        let config = MassWriteConfig {
            count_threshold: 10,
            window_seconds: 60,
        };
        let detector = MassWriteDetector::new(&config);
        let mut state = DetectorState::new();

        let base_ts = 1_000_000_000_000u64;

        for i in 0..20 {
            let mut event = FileEvent::new(
                i, 100, "test.exe".into(),
                OperationType::Write,
                format!(r"C:\test\file{}.txt", i),
            );
            event.timestamp_ns = base_ts + (i * 100_000_000); // 100ms apart
            let _ = detector.evaluate(&event, &mut state);
        }

        // The last evaluation should have a positive score
        let mut final_event = FileEvent::new(
            21, 100, "test.exe".into(),
            OperationType::Write, r"C:\test\final.txt".into(),
        );
        final_event.timestamp_ns = base_ts + (20 * 100_000_000);
        let result = detector.evaluate(&final_event, &mut state);
        assert!(result.score > 0.0);
    }
}
