"""
SentinelGuard ML Training Pipeline

Trains a Random Forest classifier on synthetic detector feature vectors
and exports the model to ONNX format for consumption by the Rust agent.

Usage:
    python train.py [--output model.onnx] [--samples 10000] [--dry-run]

The model input is a vector of 7 detector scores (each in [0.0, 1.0]).
The model output is the probability of the "ransomware" class.

Feature engineering rationale:
    - Each detector in the agent produces a score representing suspicious
      behavior along one dimension. The ML model learns non-linear
      combinations of these signals that distinguish true ransomware
      behavior from benign high-activity processes (e.g., backup software,
      build tools, batch file processors).
    - Synthetic training data is generated with realistic distributions
      based on known ransomware behavior patterns and typical benign
      process profiles.
"""

import argparse
import os
import sys

import numpy as np
from sklearn.ensemble import RandomForestClassifier
from sklearn.model_selection import train_test_split
from sklearn.metrics import classification_report, roc_auc_score

from features import FEATURE_NAMES, NUM_FEATURES


def generate_synthetic_data(n_samples: int = 10000, seed: int = 42) -> tuple:
    """
    Generate synthetic training data with realistic distributions.

    Ransomware samples: Multiple detectors fire with correlated high scores.
    Benign samples: Occasional single-detector spikes (e.g., legitimate
    encryption tools will trigger entropy_spike alone).

    Returns:
        X: Feature matrix of shape (n_samples, 7)
        y: Labels array of shape (n_samples,) with 0=benign, 1=ransomware
    """
    rng = np.random.RandomState(seed)
    n_half = n_samples // 2

    # --- Ransomware samples ---
    # Multiple correlated high scores
    ransomware_base = rng.beta(5, 2, size=(n_half, NUM_FEATURES))

    # Entropy spike is almost always high in ransomware
    ransomware_base[:, 0] = rng.beta(8, 1.5, size=n_half)

    # Mass write is usually high
    ransomware_base[:, 1] = rng.beta(6, 2, size=n_half)

    # Mass rename/delete is high
    ransomware_base[:, 2] = rng.beta(5, 2, size=n_half)

    # Ransom note appears in ~70% of ransomware
    mask_note = rng.random(n_half) < 0.7
    ransomware_base[~mask_note, 3] = 0.0

    # Shadow copy deletion in ~40% of ransomware
    mask_shadow = rng.random(n_half) < 0.4
    ransomware_base[~mask_shadow, 4] = 0.0

    # Extension explosion is high in most ransomware
    ransomware_base[:, 6] = rng.beta(6, 2, size=n_half)

    ransomware_base = np.clip(ransomware_base, 0.0, 1.0)

    # --- Benign samples ---
    # Mostly low scores with occasional single-spike
    benign_base = rng.beta(1.5, 8, size=(n_half, NUM_FEATURES))

    # Some benign processes have high entropy (compression tools)
    entropy_noise = rng.random(n_half) < 0.1
    benign_base[entropy_noise, 0] = rng.beta(6, 2, size=entropy_noise.sum())

    # Build tools may write many files
    write_noise = rng.random(n_half) < 0.08
    benign_base[write_noise, 1] = rng.beta(4, 3, size=write_noise.sum())

    # Backup software touches many extensions
    ext_noise = rng.random(n_half) < 0.05
    benign_base[ext_noise, 5] = rng.beta(4, 3, size=ext_noise.sum())

    benign_base = np.clip(benign_base, 0.0, 1.0)

    # Combine
    X = np.vstack([ransomware_base, benign_base]).astype(np.float32)
    y = np.array([1] * n_half + [0] * n_half, dtype=np.int64)

    # Shuffle
    indices = rng.permutation(n_samples)
    X = X[indices]
    y = y[indices]

    return X, y


def train_model(X_train, y_train) -> RandomForestClassifier:
    """Train a Random Forest classifier."""
    model = RandomForestClassifier(
        n_estimators=100,
        max_depth=10,
        min_samples_split=5,
        min_samples_leaf=2,
        random_state=42,
        n_jobs=-1,
        class_weight="balanced",
    )
    model.fit(X_train, y_train)
    return model


def export_to_onnx(model: RandomForestClassifier, output_path: str):
    """Export the trained model to ONNX format."""
    try:
        from skl2onnx import convert_sklearn
        from skl2onnx.common.data_types import FloatTensorType
    except ImportError:
        print("ERROR: skl2onnx is required for ONNX export.")
        print("Install with: pip install skl2onnx")
        sys.exit(1)

    initial_type = [("input", FloatTensorType([None, NUM_FEATURES]))]

    onnx_model = convert_sklearn(
        model,
        "sentinelguard_ransomware_detector",
        initial_types=initial_type,
        target_opset=13,
        options={id(model): {"zipmap": False}},
    )

    # Rename output to match the Rust agent's expectations
    for output in onnx_model.graph.output:
        if "probabilities" in output.name or "output" in output.name:
            pass  # Keep existing names
        elif output.name == "label":
            continue  # Skip the label output

    with open(output_path, "wb") as f:
        f.write(onnx_model.SerializeToString())

    print(f"ONNX model saved to: {output_path}")

    # Verify the exported model
    try:
        import onnxruntime as ort
        session = ort.InferenceSession(output_path)
        inputs = session.get_inputs()
        outputs = session.get_outputs()
        print(f"  Inputs:  {[(i.name, i.shape) for i in inputs]}")
        print(f"  Outputs: {[(o.name, o.shape) for o in outputs]}")

        # Test inference
        test_input = np.zeros((1, NUM_FEATURES), dtype=np.float32)
        result = session.run(None, {inputs[0].name: test_input})
        print(f"  Test inference (all zeros): {result}")
    except ImportError:
        print("  (onnxruntime not installed, skipping verification)")


def main():
    parser = argparse.ArgumentParser(
        description="SentinelGuard ML Training Pipeline"
    )
    parser.add_argument(
        "--output", default="model.onnx",
        help="Output ONNX model path (default: model.onnx)"
    )
    parser.add_argument(
        "--samples", type=int, default=10000,
        help="Number of synthetic training samples (default: 10000)"
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Train and evaluate but don't export"
    )
    args = parser.parse_args()

    print(f"SentinelGuard ML Training Pipeline")
    print(f"=" * 50)
    print(f"Features ({NUM_FEATURES}):")
    for i, name in enumerate(FEATURE_NAMES):
        print(f"  [{i}] {name}")
    print()

    # Generate data
    print(f"Generating {args.samples} synthetic samples...")
    X, y = generate_synthetic_data(n_samples=args.samples)
    print(f"  Ransomware: {np.sum(y == 1)}, Benign: {np.sum(y == 0)}")

    # Split
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42, stratify=y,
    )
    print(f"  Train: {len(X_train)}, Test: {len(X_test)}")

    # Train
    print("\nTraining Random Forest...")
    model = train_model(X_train, y_train)
    print("  Training complete.")

    # Evaluate
    print("\nEvaluation on test set:")
    y_pred = model.predict(X_test)
    y_proba = model.predict_proba(X_test)[:, 1]

    print(classification_report(y_test, y_pred, target_names=["benign", "ransomware"]))
    auc = roc_auc_score(y_test, y_proba)
    print(f"  ROC AUC: {auc:.4f}")

    # Feature importance
    print("\nFeature importance:")
    importances = model.feature_importances_
    for name, imp in sorted(zip(FEATURE_NAMES, importances), key=lambda x: -x[1]):
        print(f"  {name}: {imp:.4f}")

    # Export
    if not args.dry_run:
        print(f"\nExporting to ONNX: {args.output}")
        export_to_onnx(model, args.output)
    else:
        print("\n(Dry run, skipping ONNX export)")

    print("\nDone.")


if __name__ == "__main__":
    main()
