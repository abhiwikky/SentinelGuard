# ML Feature Reference

## Feature Set

The ML correlation engine uses the following features derived from detector outputs and event patterns:

### 1. Detector Scores (7 features)
- `entropy_score`: Entropy spike detector output (0.0 - 1.0)
- `mass_write_score`: Mass write detector output (0.0 - 1.0)
- `mass_rename_delete_score`: Rename/delete detector output (0.0 - 1.0)
- `ransom_note_score`: Ransom note detector output (0.0 - 1.0)
- `shadow_copy_score`: Shadow copy deletion detector output (0.0 - 1.0)
- `process_behavior_score`: Process behavior detector output (0.0 - 1.0)
- `file_extension_score`: File extension detector output (0.0 - 1.0)

### 2. Temporal Features (5 features)
- `event_rate`: Events per second over last 10 seconds
- `write_rate`: File writes per second
- `rename_rate`: File renames per second
- `delete_rate`: File deletes per second
- `burst_interval`: Time between activity bursts

### 3. Process Features (3 features)
- `process_age`: Time since process started (seconds)
- `file_diversity`: Number of unique file extensions accessed
- `directory_diversity`: Number of unique directories accessed

### 4. Entropy Features (3 features)
- `avg_entropy`: Average entropy of written files
- `entropy_variance`: Variance in entropy values
- `entropy_trend`: Increasing/decreasing trend

### 5. Aggregate Features (2 features)
- `detector_agreement`: Number of detectors firing simultaneously
- `risk_momentum`: Rate of risk score increase

## Feature Engineering

### Normalization
All features are normalized to [0, 1] range using min-max scaling.

### Temporal Windows
- Short window: 1 second
- Medium window: 10 seconds
- Long window: 60 seconds

## Model Architecture

### Option 1: Random Forest
- 100 trees
- Max depth: 20
- Min samples split: 5

### Option 2: LightGBM
- Learning rate: 0.1
- Num leaves: 31
- Max depth: 10

### Option 3: Neural Network (LSTM)
- Input: Sequential feature vectors (last 60 seconds)
- Hidden layers: 2 LSTM layers (64 units each)
- Output: Binary classification (ransomware probability)

## Training Data

### Positive Samples
- Real ransomware samples (WannaCry, Locky, Ryuk, etc.)
- Simulated ransomware behavior
- Historical attack logs

### Negative Samples
- Normal file operations
- Encryption software (legitimate)
- Backup operations
- File compression

## Model Export

Models are exported to ONNX format for inference in Rust:
```python
import onnx
onnx_model = convert_model_to_onnx(model)
onnx.save(onnx_model, "ransomware_model.onnx")
```

## Inference

In Rust agent:
```rust
let features = extract_features(detector_scores, events);
let ml_score = onnx_session.run(features)?;
```

## Threshold Tuning

Default threshold: 0.7

Adjust based on:
- False positive rate
- Detection latency requirements
- Business impact

