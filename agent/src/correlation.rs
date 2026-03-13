//
// ML Correlation Engine with ONNX Runtime
//

use anyhow::{anyhow, Context, Result};
use ort::memory::Allocator;
use ort::session::Session;
use ort::value::{DynMapValueType, DynSequenceValueType, TensorRef};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

use crate::detectors::DetectorScores;

const FEATURE_COUNT: usize = 15;

pub struct CorrelationEngine {
    session: Arc<Mutex<Session>>,
}

impl CorrelationEngine {
    pub async fn new(model_path: &PathBuf) -> Result<Self> {
        if !model_path.exists() {
            return Err(anyhow!(
                "ONNX model not found at {}",
                model_path.display()
            ));
        }

        let path = model_path.clone();
        let session = std::panic::catch_unwind(move || -> Result<Session> {
            Session::builder()?
                .commit_from_file(&path)
                .context("Failed to load ONNX model")
        })
        .map_err(|_| anyhow!("ONNX runtime panicked while loading {}", model_path.display()))??;

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
        })
    }

    pub async fn infer(&self, scores: &DetectorScores) -> Result<f32> {
        let features = self.build_feature_vector(scores);
        debug!("Running ONNX inference with {} features", features.len());
        self.run_onnx_inference(&features).await
    }

    fn build_feature_vector(&self, scores: &DetectorScores) -> Vec<f32> {
        vec![
            scores.entropy_score,
            scores.mass_write_score,
            scores.mass_rename_delete_score,
            scores.ransom_note_score,
            scores.shadow_copy_score,
            scores.process_behavior_score,
            scores.file_extension_score,
            scores.event_rate,
            scores.avg_entropy_per_sec,
            scores.rename_delete_freq,
            scores.burst_interval,
            scores.num_detectors_firing,
            scores.file_diversity,
            scores.bytes_written_per_sec,
            scores.unique_extensions,
        ]
    }

    async fn run_onnx_inference(&self, features: &[f32]) -> Result<f32> {
        use ort::inputs;

        if features.len() != FEATURE_COUNT {
            return Err(anyhow!(
                "Expected {} features for ONNX inference, got {}",
                FEATURE_COUNT,
                features.len()
            ));
        }

        let mut session = self.session.lock().await;
        let input_value = inputs![TensorRef::from_array_view(([1, features.len()], features))?];
        let outputs = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| session.run(input_value)))
            .map_err(|_| anyhow!("ONNX runtime panicked during inference"))??;

        for candidate in outputs.values() {
            if let Ok((_, tensor)) = candidate.try_extract_tensor::<f32>() {
                if let Some(score) = tensor.iter().copied().last() {
                    return Ok(score.clamp(0.0, 1.0));
                }
            }
        }

        let allocator = Allocator::default();
        for candidate in outputs.values() {
            if let Ok(sequence) = candidate.downcast_ref::<DynSequenceValueType>() {
                let maps = sequence.try_extract_sequence::<DynMapValueType>(&allocator)?;
                for map in maps {
                    if let Ok(probabilities) = map.try_extract_map::<i64, f32>() {
                        if let Some(score) = probabilities
                            .get(&1)
                            .copied()
                            .or_else(|| probabilities.values().copied().max_by(f32::total_cmp))
                        {
                            return Ok(score.clamp(0.0, 1.0));
                        }
                    }
                }
            }
        }

        for candidate in outputs.values() {
            if let Ok((_, tensor)) = candidate.try_extract_tensor::<i64>() {
                if let Some(label) = tensor.iter().copied().last() {
                    return Ok(if label > 0 { 1.0 } else { 0.0 });
                }
            }
        }

        Err(anyhow!("Failed to extract ONNX output score"))
    }
}
