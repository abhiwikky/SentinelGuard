"""
SentinelGuard ML Training Pipeline

Trains a Gradient Boosted classifier on synthetic detector feature vectors
and exports the model to ONNX format for consumption by the Rust agent.

Usage:
    python train.py [--output model.onnx] [--samples 50000] [--dry-run]

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
from sklearn.neural_network import MLPClassifier
from sklearn.model_selection import train_test_split
from sklearn.metrics import classification_report, roc_auc_score

from features import FEATURE_NAMES, NUM_FEATURES


def generate_synthetic_data(n_samples: int = 50000, seed: int = 42) -> tuple:
    """
    Generate synthetic training data with realistic distributions.

    Ransomware samples: Multiple detectors fire with correlated high scores.
    Benign samples: Richly diverse profiles covering idle services, browsers,
    developer tools, antivirus scanners, backup software, installers, and more.
    Each benign category is modeled after real-world process behavior to teach
    the model what normal looks like.

    Returns:
        X: Feature matrix of shape (n_samples, 7)
        y: Labels array of shape (n_samples,) with 0=benign, 1=ransomware
    """
    rng = np.random.RandomState(seed)
    n_half = n_samples // 2

    # --- Ransomware samples ---
    # Start from ZEROS and build up each sub-type's profile explicitly.
    # This ensures each category has the right feature signature without
    # contamination from a high base initialization.
    ransomware_base = np.zeros((n_half, NUM_FEATURES), dtype=np.float64)

    # Assign ransomware sub-types for realistic diversity
    ransomware_type = rng.choice(
        ["full_crypto", "early_stage", "script_based", "wiper"],
        size=n_half,
        p=[0.40, 0.25, 0.20, 0.15],
    )

    # --- Full crypto ransomware (40%): classic pattern, all signals high ---
    full_mask = ransomware_type == "full_crypto"
    n_full = full_mask.sum()
    if n_full > 0:
        ransomware_base[full_mask, 0] = rng.beta(8, 1.5, size=n_full)   # entropy: high
        ransomware_base[full_mask, 1] = rng.beta(6, 2, size=n_full)     # mass_write: high
        ransomware_base[full_mask, 2] = rng.beta(5, 2, size=n_full)     # mass_rename: high
        ransomware_base[full_mask, 5] = rng.beta(5, 2, size=n_full)     # process_behavior: high
        ransomware_base[full_mask, 6] = rng.beta(6, 2, size=n_full)     # ext_explosion: high

    # --- Early-stage / pre-encryption (25%): no entropy yet, but mass ops + ransom note ---
    # This matches the simulate_attacks.ps1 pattern where files are renamed/written
    # and ransom notes dropped BEFORE actual encryption begins.
    early_mask = ransomware_type == "early_stage"
    n_early = early_mask.sum()
    if n_early > 0:
        ransomware_base[early_mask, 0] = rng.beta(1.2, 10, size=n_early)  # entropy: near zero
        ransomware_base[early_mask, 1] = rng.beta(7, 1.5, size=n_early)   # mass_write: very high
        ransomware_base[early_mask, 2] = rng.beta(7, 1.5, size=n_early)   # mass_rename: very high
        ransomware_base[early_mask, 5] = rng.beta(4, 3, size=n_early)     # process_behavior: moderate
        # entropy, shadow_copy, ext_explosion stay near zero

    # --- Script-based ransomware (20%): PowerShell/batch, high mass ops ---
    script_mask = ransomware_type == "script_based"
    n_script = script_mask.sum()
    if n_script > 0:
        ransomware_base[script_mask, 0] = rng.beta(1.5, 6, size=n_script)  # entropy: low
        ransomware_base[script_mask, 1] = rng.beta(7, 2, size=n_script)    # mass_write: very high
        ransomware_base[script_mask, 2] = rng.beta(6, 2, size=n_script)    # mass_rename: high
        ransomware_base[script_mask, 5] = rng.beta(4, 3, size=n_script)    # process_behavior: moderate
        ransomware_base[script_mask, 6] = rng.beta(3, 4, size=n_script)    # ext_explosion: moderate

    # --- Wiper-style (15%): mass delete/rename, no encryption ---
    wiper_mask = ransomware_type == "wiper"
    n_wiper = wiper_mask.sum()
    if n_wiper > 0:
        ransomware_base[wiper_mask, 0] = rng.beta(1.1, 10, size=n_wiper)  # entropy: near zero
        ransomware_base[wiper_mask, 1] = rng.beta(5, 2, size=n_wiper)     # mass_write: high
        ransomware_base[wiper_mask, 2] = rng.beta(8, 1.5, size=n_wiper)   # mass_rename/delete: very high
        ransomware_base[wiper_mask, 5] = rng.beta(5, 2, size=n_wiper)     # process_behavior: high

    # Ransom note appears in ~70% of ransomware (across ALL types)
    mask_note = rng.random(n_half) < 0.7
    ransomware_base[mask_note, 3] = rng.beta(6, 2, size=mask_note.sum())

    # Shadow copy deletion in ~40% of ransomware
    mask_shadow = rng.random(n_half) < 0.4
    ransomware_base[mask_shadow, 4] = rng.beta(5, 2, size=mask_shadow.sum())

    ransomware_base = np.clip(ransomware_base, 0.0, 1.0)

    # --- Benign samples ---
    # Realistic distribution of benign process profiles.
    # Each sub-population models a known category of safe software.
    benign_base = np.zeros((n_half, NUM_FEATURES), dtype=np.float64)

    # Assign each benign sample to a category
    categories = rng.choice(
        ["idle", "light", "browser", "developer", "av_scanner",
         "compression", "build_tool", "backup", "installer", "adversarial"],
        size=n_half,
        p=[0.30, 0.15, 0.12, 0.08, 0.05, 0.05, 0.07, 0.04, 0.04, 0.10],
    )

    # --- Idle/system processes (30%): all zeros ---
    # svchost.exe, csrss.exe, System, etc.
    # (already zeros, nothing to do)

    # --- Light-activity processes (15%): browsers/editors with tiny noise ---
    light_mask = categories == "light"
    n_light = light_mask.sum()
    if n_light > 0:
        benign_base[light_mask] = rng.beta(1.2, 15, size=(n_light, NUM_FEATURES))

    # --- Browsers (12%): moderate process_behavior (many extensions/dirs) ---
    # Browsers like msedge.exe, chrome.exe touch .tmp, .cache, .json, .js,
    # .css, .html, .woff, .png, etc. — this triggers process_behavior detector.
    browser_mask = categories == "browser"
    n_browser = browser_mask.sum()
    if n_browser > 0:
        benign_base[browser_mask, 5] = rng.beta(3, 4, size=n_browser)  # process_behavior: moderate
        benign_base[browser_mask, 1] = rng.beta(1.5, 8, size=n_browser)  # light writes
        benign_base[browser_mask, 6] = rng.beta(1.5, 6, size=n_browser)  # some extension variety
        # All other detectors near zero (no entropy spike, no ransom, no shadow)

    # --- Developer tools (8%): mass writes + some extension + process_behavior ---
    dev_mask = categories == "developer"
    n_dev = dev_mask.sum()
    if n_dev > 0:
        benign_base[dev_mask, 1] = rng.beta(4, 4, size=n_dev)  # mass_write: moderate
        benign_base[dev_mask, 5] = rng.beta(3, 3, size=n_dev)  # process_behavior: moderate
        benign_base[dev_mask, 6] = rng.beta(2, 5, size=n_dev)  # extension variety
        benign_base[dev_mask, 2] = rng.beta(1.5, 8, size=n_dev)  # light renames

    # --- AV scanners (5%): high directory traversal, moderate process_behavior ---
    av_mask = categories == "av_scanner"
    n_av = av_mask.sum()
    if n_av > 0:
        benign_base[av_mask, 5] = rng.beta(4, 3, size=n_av)  # broad directory access
        benign_base[av_mask, 1] = rng.beta(2, 6, size=n_av)  # some quarantine writes

    # --- Compression / encryption tools (5%): high entropy only ---
    entropy_mask = categories == "compression"
    n_entropy = entropy_mask.sum()
    if n_entropy > 0:
        benign_base[entropy_mask, 0] = rng.beta(6, 2, size=n_entropy)
        benign_base[entropy_mask, 1] = rng.beta(2, 5, size=n_entropy)

    # --- Build tools (7%): compilers, bundlers with mass writes ---
    build_mask = categories == "build_tool"
    n_build = build_mask.sum()
    if n_build > 0:
        benign_base[build_mask, 1] = rng.beta(5, 3, size=n_build)
        benign_base[build_mask, 5] = rng.beta(3, 4, size=n_build)
        benign_base[build_mask, 6] = rng.beta(2, 5, size=n_build)

    # --- Backup / sync software (4%): broad directory + extension access ---
    backup_mask = categories == "backup"
    n_backup = backup_mask.sum()
    if n_backup > 0:
        benign_base[backup_mask, 1] = rng.beta(4, 3, size=n_backup)
        benign_base[backup_mask, 5] = rng.beta(5, 3, size=n_backup)
        benign_base[backup_mask, 6] = rng.beta(3, 4, size=n_backup)

    # --- Installers (4%): mass writes + renames ---
    installer_mask = categories == "installer"
    n_installer = installer_mask.sum()
    if n_installer > 0:
        benign_base[installer_mask, 1] = rng.beta(5, 2, size=n_installer)
        benign_base[installer_mask, 2] = rng.beta(3, 4, size=n_installer)
        benign_base[installer_mask, 5] = rng.beta(3, 5, size=n_installer)

    # --- Adversarial benign (10%): single detector elevated, rest near zero ---
    # These teach the model that a SINGLE high detector does NOT equal ransomware.
    # Critical for preventing false positives from browsers/build tools.
    # NOTE: Only elevate detectors that benign processes can legitimately trigger.
    # ransom_note (3) and shadow_copy (4) are ransomware-exclusive signals —
    # no benign process creates ransom notes or deletes shadow copies.
    adv_mask = categories == "adversarial"
    n_adv = adv_mask.sum()
    benign_safe_detectors = [0, 1, 2, 5, 6]  # entropy, mass_write, mass_rename, process_behavior, ext_explosion
    if n_adv > 0:
        # Choose a random single SAFE detector to elevate for each sample
        elevated_detector = rng.choice(benign_safe_detectors, size=n_adv)
        for i, det_idx in enumerate(elevated_detector):
            adv_idx = np.where(adv_mask)[0][i]
            benign_base[adv_idx, det_idx] = rng.beta(5, 2)
            # Add tiny noise to 0-2 other SAFE detectors
            noise_count = rng.randint(0, 3)
            if noise_count > 0:
                noise_dets = rng.choice(
                    [j for j in benign_safe_detectors if j != det_idx],
                    size=min(noise_count, len(benign_safe_detectors) - 1),
                    replace=False,
                )
                for nd in noise_dets:
                    benign_base[adv_idx, nd] = rng.beta(1.5, 10)

    # CRITICAL: Force ransomware-exclusive signals to zero for ALL benign samples.
    # ransom_note (col 3) and shadow_copy (col 4) should NEVER appear in benign
    # data — no legitimate process creates ransom notes or deletes shadow copies.
    # Without this, beta noise from the "light" category leaks into these columns.
    benign_base[:, 3] = 0.0  # ransom_note
    benign_base[:, 4] = 0.0  # shadow_copy

    benign_base = np.clip(benign_base, 0.0, 1.0)

    # Combine
    X = np.vstack([ransomware_base, benign_base]).astype(np.float32)
    y = np.array([1] * n_half + [0] * n_half, dtype=np.int64)

    # Shuffle
    indices = rng.permutation(n_samples)
    X = X[indices]
    y = y[indices]

    return X, y


def train_model(X_train, y_train):
    """Train an MLP classifier for ransomware detection.

    MLP handles the non-linear feature interactions (e.g. mass_write +
    ransom_note without entropy) much better than tree-based models which
    tend to overfit to the dominant full-crypto ransomware pattern.
    """
    model = MLPClassifier(
        hidden_layer_sizes=(64, 32),
        activation="relu",
        max_iter=500,
        early_stopping=True,
        validation_fraction=0.1,
        random_state=42,
    )
    model.fit(X_train, y_train)
    return model


def export_to_onnx(model, output_path: str):
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

        # Test inference with critical cases
        test_cases = {
            "all_zeros": np.zeros((1, NUM_FEATURES), dtype=np.float32),
            "browser_like": np.array([[0.0, 0.02, 0.0, 0.0, 0.0, 0.4, 0.05]], dtype=np.float32),
            "ransomware_like": np.array([[0.9, 0.8, 0.7, 0.5, 0.0, 0.6, 0.8]], dtype=np.float32),
        }
        for name, test_input in test_cases.items():
            result = session.run(None, {inputs[0].name: test_input})
            probs = result[1][0] if len(result) > 1 else result[0][0]
            ransomware_prob = probs[1] if hasattr(probs, '__len__') and len(probs) >= 2 else float(probs)
            print(f"  Test [{name}]: P(ransomware) = {ransomware_prob:.4f}")
            if name == "all_zeros":
                assert ransomware_prob < 0.05, f"FAIL: all-zeros should be <5%, got {ransomware_prob:.4f}"
            elif name == "browser_like":
                assert ransomware_prob < 0.15, f"FAIL: browser profile should be <15%, got {ransomware_prob:.4f}"
            elif name == "ransomware_like":
                assert ransomware_prob > 0.85, f"FAIL: ransomware profile should be >85%, got {ransomware_prob:.4f}"
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
        "--samples", type=int, default=50000,
        help="Number of synthetic training samples (default: 50000)"
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
    try:
        importances = model.feature_importances_
        # Normalize to sum to 1 for comparability
        importances = importances / importances.sum()
        for name, imp in sorted(zip(FEATURE_NAMES, importances), key=lambda x: -x[1]):
            print(f"  {name}: {imp:.4f}")
    except AttributeError:
        print("  (feature importance not available for this model type)")

    # Export
    if not args.dry_run:
        print(f"\nExporting to ONNX: {args.output}")
        export_to_onnx(model, args.output)
    else:
        print("\n(Dry run, skipping ONNX export)")

    print("\nDone.")


if __name__ == "__main__":
    main()
