# SentinelGuard ML Training

This directory contains the machine learning training pipeline for SentinelGuard.

## Setup

```bash
pip install -r requirements.txt
```

## Training

```bash
python train_model.py
```

This will:
1. Generate synthetic training data (in production, use real event logs)
2. Train a RandomForest classifier
3. Train a LightGBM model (optional)
4. Export the model to ONNX format
5. Save feature scaler for inference

## Output

- `models/sentinelguard_model.onnx` - ONNX model for Rust inference
- `models/scaler.joblib` - Feature scaler
- `models/random_forest.joblib` - Original scikit-learn model

## Features

The model uses 15 features:
- 7 detector scores (entropy, mass write, rename/delete, ransom note, shadow copy, process behavior, file extension)
- 8 derived features (event rate, entropy per second, rename/delete frequency, burst intervals, etc.)

## Production Training

For production, replace `generate_synthetic_data()` with:
- Loading from SQLite database
- Replaying real-world ransomware samples
- Using labeled event logs from honeypots

