//
// ML Correlation Engine with ONNX Runtime
//

use anyhow::{Result, Context};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, warn};
use ort::{Session, Value, Environment, ExecutionProvider};
use crate::detectors::DetectorScores;

pub struct CorrelationEngine {
    session: Arc<Session>,
    feature_count: usize,
}

impl CorrelationEngine {
    pub async fn new(model_path: &PathBuf) -> Result<Self> {
        debug!("Loading ML model from: {:?}", model_path);
        
        // Initialize ONNX Runtime environment
        let environment = Arc::new(
            Environment::builder()
                .with_name("SentinelGuard")
                .with_execution_providers([ExecutionProvider::CPU(Default::default())])
                .build()
                .context("Failed to create ONNX Runtime environment")?
        );

        // Load ONNX model
        let session = if model_path.exists() {
            Session::builder(&environment)?
                .with_model_from_file(model_path)
                .context("Failed to load ONNX model")?
        } else {
            warn!("ONNX model not found at {:?}, using fallback scoring", model_path);
            // Return a dummy session - we'll use fallback in infer()
            return Ok(Self {
                session: Arc::new(
                    Session::builder(&environment)?
                        .commit_from_memory(&[])
                        .context("Failed to create dummy session")?
                ),
                feature_count: 15, // Expected feature count
            });
        };

        // Get input shape to determine feature count
        let input_shape = session.inputs[0].shape.as_ref()
            .and_then(|s| s.last())
            .and_then(|d| d.as_dim_value())
            .unwrap_or(15);

        debug!("ONNX model loaded, input features: {}", input_shape);

        Ok(Self {
            session: Arc::new(session),
            feature_count: input_shape,
        })
    }

    pub async fn infer(&self, scores: &DetectorScores) -> Result<f32> {
        // Extract base features from detector scores
        let mut features = vec![
            scores.entropy_score,
            scores.mass_write_score,
            scores.mass_rename_delete_score,
            scores.ransom_note_score,
            scores.shadow_copy_score,
            scores.process_behavior_score,
            scores.file_extension_score,
        ];

        // Add derived features (simplified - in production, compute from event history)
        // These would normally come from process statistics
        features.push(0.0); // event_rate
        features.push(scores.entropy_score * 2.0); // avg_entropy_per_sec (simplified)
        features.push(scores.mass_rename_delete_score * 10.0); // rename_delete_freq
        features.push(1.0); // burst_interval
        features.push(0.0); // num_detectors_firing (would count active detectors)
        features.push(0.0); // file_diversity
        features.push(scores.mass_write_score * 1000.0); // bytes_written_per_sec
        features.push(0.0); // unique_extensions

        // Ensure we have the right number of features
        while features.len() < self.feature_count {
            features.push(0.0);
        }
        features.truncate(self.feature_count);

        // Try ONNX inference if model is loaded
        if self.session.inputs.len() > 0 {
            match self.run_onnx_inference(&features) {
                Ok(score) => {
                    debug!("ONNX inference score: {:.4}", score);
                    return Ok(score);
                }
                Err(e) => {
                    warn!("ONNX inference failed: {}, using fallback", e);
                }
            }
        }

        // Fallback: weighted average
        let weights = vec![0.2, 0.25, 0.2, 0.15, 0.1, 0.05, 0.05];
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for (feature, weight) in features.iter().take(7).zip(weights.iter()) {
            weighted_sum += feature * weight;
            total_weight += weight;
        }

        let ml_score = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        };

        debug!("Fallback ML inference score: {:.4}", ml_score);
        Ok(ml_score)
    }

    fn run_onnx_inference(&self, features: &[f32]) -> Result<f32> {
        use ort::inputs;
        use ort::ndarray::Array2;
        
        // Prepare input tensor as 2D array [1, features.len()]
        let input_array = Array2::from_shape_vec((1, features.len()), features.to_vec())
            .context("Failed to create input array")?;
        
        // Create input value
        let input_value = inputs!["float_input" => input_array]?;

        // Run inference
        let outputs = self.session.run(input_value)?;
        
        // Extract output (assuming binary classification, get probability of class 1)
        let output = outputs["output"]
            .try_extract_tensor::<f32>()
            .or_else(|_| {
                // Try alternative output name
                outputs.values().next()
                    .and_then(|v| v.try_extract_tensor::<f32>().ok())
                    .ok_or_else(|| anyhow::anyhow!("Failed to extract output tensor"))
            })?;
        
        let output_slice = output.view().as_slice()
            .ok_or_else(|| anyhow::anyhow!("Failed to get output slice"))?;
        
        // If output is shape [1, 2], take the second value (malicious probability)
        // If output is shape [1, 1], use that value directly
        let score = if output_slice.len() > 1 {
            output_slice[1] // Probability of malicious class
        } else {
            output_slice[0] // Single output value
        };

        Ok(score.clamp(0.0, 1.0))
    }
}

