//! SentinelGuard ONNX Inference Module
//!
//! Loads an ONNX model and runs inference on aggregated detector features
//! to produce a final risk score. Falls back to weighted average if
//! model loading fails.

use crate::config::InferenceConfig;
use crate::events::AggregatedScore;
use anyhow::Result;
use ort::session::{builder::GraphOptimizationLevel, Session};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info, warn};

/// ONNX model wrapper for ransomware risk inference
pub struct InferenceEngine {
    session: Option<Arc<Mutex<Session>>>,
    num_features: usize,
    #[allow(dead_code)]
    fallback_enabled: bool,
}

impl InferenceEngine {
    /// Create a new inference engine, loading the ONNX model from disk.
    /// If loading fails and fallback is enabled, logs a warning and continues.
    pub fn new(config: &InferenceConfig) -> Result<Self> {
        let model_path = Path::new(&config.model_path);

        let session = if model_path.exists() {
            match Self::load_model(model_path) {
                Ok(sess) => {
                    info!("ONNX model loaded from {}", config.model_path);
                    Some(Arc::new(Mutex::new(sess)))
                }
                Err(e) => {
                    if config.fallback_enabled {
                        warn!(
                            "Failed to load ONNX model: {}. Using fallback scoring.",
                            e
                        );
                        None
                    } else {
                        return Err(anyhow::anyhow!("Failed to load ONNX model: {}", e));
                    }
                }
            }
        } else if config.fallback_enabled {
            warn!(
                "ONNX model not found at {}. Using fallback scoring.",
                config.model_path
            );
            None
        } else {
            anyhow::bail!(
                "ONNX model not found at {} and fallback is disabled",
                config.model_path
            );
        };

        Ok(Self {
            session,
            num_features: config.num_features,
            fallback_enabled: config.fallback_enabled,
        })
    }

    fn load_model(path: &Path) -> Result<Session> {
        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("Failed to create ONNX session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("Failed to set optimization level: {}", e))?
            .commit_from_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to load ONNX model file: {}", e))?;

        Ok(session)
    }

    /// Check if the model is loaded
    pub fn is_model_loaded(&self) -> bool {
        self.session.is_some()
    }

    /// Run inference on an aggregated score to produce a final ML risk score.
    /// Updates the AggregatedScore in place.
    pub fn predict(&self, score: &mut AggregatedScore) -> Result<()> {
        let features = self.extract_features(score);

        if let Some(session) = &self.session {
            let mut session = session.lock();
            match self.run_inference(&mut session, &features) {
                Ok(ml_score) => {
                    score.ml_score = ml_score;
                    // Blend heuristic and ML, but ensure a high-confidence ML score
                    // can override a diluted heuristic score, AND a high-confidence 
                    // heuristic score isn't completely suppressed by an uncertain ML model.
                    let blended = score.weighted_score * 0.4 + score.ml_score * 0.6;
                    score.final_score = blended
                        .max(score.ml_score * 0.9)
                        .max(score.weighted_score * 0.85)
                        .min(1.0);
                }
                Err(e) => {
                    error!("ML inference failed: {}. Using weighted score.", e);
                    score.ml_score = 0.0;
                    score.final_score = score.weighted_score;
                }
            }
        } else {
            score.ml_score = 0.0;
            score.final_score = score.weighted_score;
        }

        Ok(())
    }

    /// Extract feature vector from aggregated detector results.
    fn extract_features(&self, score: &AggregatedScore) -> Vec<f32> {
        let detector_names = [
            "entropy_spike",
            "mass_write",
            "mass_rename_delete",
            "ransom_note",
            "shadow_copy",
            "process_behavior",
            "extension_explosion",
        ];

        detector_names
            .iter()
            .map(|name| {
                score
                    .detector_results
                    .iter()
                    .filter(|r| r.detector_name == *name)
                    .map(|r| r.score as f32)
                    .fold(0.0f32, f32::max)
            })
            .collect()
    }

    fn run_inference(&self, session: &mut Session, features: &[f32]) -> Result<f64> {
        // Create input tensor using (shape, Vec) tuple that ort v2 accepts
        let input_data = features.to_vec();
        let input_value = ort::value::Value::from_array(
            ([1usize, self.num_features], input_data)
        )
        .map_err(|e| anyhow::anyhow!("Failed to create input tensor: {}", e))?;

        // ort::inputs! returns a Vec, not a Result
        let inputs = ort::inputs!["input" => input_value];

        let outputs = session
            .run(inputs)
            .map_err(|e| anyhow::anyhow!("ONNX inference failed: {}", e))?;

        // The model outputs a probability for the "ransomware" class
        let output = outputs
            .get("probabilities")
            .or_else(|| outputs.get("output"))
            .ok_or_else(|| anyhow::anyhow!("Model output not found"))?;

        let (_shape, data) = output
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow::anyhow!("Failed to extract output tensor: {}", e))?;

        // Handle both [1, 2] and [1, 1] output shapes
        let ml_score: f64 = if data.len() >= 2 {
            data[1] as f64 // Probability of class 1 (ransomware)
        } else if data.len() == 1 {
            data[0] as f64
        } else {
            warn!("Unexpected output tensor shape, using 0.0");
            0.0
        };

        Ok(ml_score.clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::DetectorResult;

    fn make_score(results: Vec<(&str, f64)>) -> AggregatedScore {
        let detector_results: Vec<DetectorResult> = results
            .into_iter()
            .map(|(name, score)| DetectorResult::new(name, score, vec![], 100))
            .collect();

        AggregatedScore {
            process_id: 100,
            process_name: "test.exe".to_string(),
            weighted_score: 0.5,
            ml_score: 0.0,
            final_score: 0.0,
            detector_results,
            window_start_ns: 0,
            window_end_ns: 0,
            total_events: 1,
        }
    }

    #[test]
    fn test_feature_extraction() {
        let config = InferenceConfig {
            model_path: "nonexistent.onnx".to_string(),
            num_features: 7,
            fallback_enabled: true,
        };
        let engine = InferenceEngine::new(&config).unwrap();

        let score = make_score(vec![
            ("entropy_spike", 0.9),
            ("mass_write", 0.5),
        ]);

        let features = engine.extract_features(&score);
        assert_eq!(features.len(), 7);
        assert!((features[0] - 0.9).abs() < 0.001);
        assert!((features[1] - 0.5).abs() < 0.001);
        assert!((features[2] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_fallback_scoring() {
        let config = InferenceConfig {
            model_path: "nonexistent.onnx".to_string(),
            num_features: 7,
            fallback_enabled: true,
        };
        let engine = InferenceEngine::new(&config).unwrap();
        assert!(!engine.is_model_loaded());

        let mut score = make_score(vec![("entropy_spike", 0.9)]);
        score.weighted_score = 0.7;
        engine.predict(&mut score).unwrap();
        assert!((score.final_score - 0.7).abs() < f64::EPSILON);
    }
}
