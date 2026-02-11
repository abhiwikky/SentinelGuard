//
// ML Correlation Engine with ONNX Runtime
//

use anyhow::{Result, Context};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};
use ort::session::Session;
use ort::value::TensorRef;
use crate::detectors::DetectorScores;

pub struct CorrelationEngine {
    session: Option<Arc<Mutex<Session>>>,
    feature_count: usize,
}

impl CorrelationEngine {
    pub async fn new(model_path: &PathBuf) -> Result<Self> {
        debug!("Loading ML model from: {:?}", model_path);

        // Load ONNX model
        let session = if model_path.exists() {
            let session = Session::builder()?
                .commit_from_file(model_path)
                .context("Failed to load ONNX model")?
            ;
            Some(Arc::new(Mutex::new(session)))
        } else {
            warn!("ONNX model not found at {:?}, using fallback scoring", model_path);
            None
        };

        // Keep a fixed feature vector size unless model metadata parsing is added.
        let input_shape = 15usize;

        debug!("ONNX model loaded, input features: {}", input_shape);

        Ok(Self {
            session,
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
        if self.session.is_some() {
            match self.run_onnx_inference(&features).await {
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

    async fn run_onnx_inference(&self, features: &[f32]) -> Result<f32> {
        use ort::inputs;

        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No ONNX session loaded"))?;
        let mut session = session.lock().await;

        // Create input tensor as 2D shape [1, features.len()].
        // Use shape+slice tuple to avoid ndarray version coupling.
        let input_value = inputs![TensorRef::from_array_view(([1, features.len()], features))?];

        // Run inference
        let outputs = session.run(input_value)?;

        // Extract output as owned data to avoid borrowing from temporary values.
        let output_values: Vec<f32> = if let Some(value) = outputs.get("output") {
            if let Ok((_, tensor)) = value.try_extract_tensor::<f32>() {
                tensor.iter().copied().collect()
            } else {
                let mut extracted: Option<Vec<f32>> = None;
                for candidate in outputs.values() {
                    if let Ok((_, tensor)) = candidate.try_extract_tensor::<f32>() {
                        extracted = Some(tensor.iter().copied().collect());
                        break;
                    }
                }
                extracted.ok_or_else(|| anyhow::anyhow!("Failed to extract output tensor"))?
            }
        } else {
            let mut extracted: Option<Vec<f32>> = None;
            for candidate in outputs.values() {
                if let Ok((_, tensor)) = candidate.try_extract_tensor::<f32>() {
                    extracted = Some(tensor.iter().copied().collect());
                    break;
                }
            }
            extracted.ok_or_else(|| anyhow::anyhow!("Failed to extract output tensor"))?
        };

        if output_values.is_empty() {
            return Err(anyhow::anyhow!("Model output tensor is empty"));
        }

        // If output is shape [1, 2], take the second value (malicious probability)
        // If output is shape [1, 1], use that value directly
        let score = if output_values.len() > 1 {
            output_values[1] // Probability of malicious class
        } else {
            output_values[0] // Single output value
        };

        Ok(score.clamp(0.0, 1.0))
    }
}

