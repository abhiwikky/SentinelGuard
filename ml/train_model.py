#!/usr/bin/env python3
"""
SentinelGuard ML Model Training Script
Trains a RandomForest/LightGBM model for ransomware detection
Exports to ONNX format for inference in Rust agent
"""

import numpy as np
import pandas as pd
from sklearn.ensemble import RandomForestClassifier
from sklearn.model_selection import train_test_split
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import classification_report, confusion_matrix, roc_auc_score
import lightgbm as lgb
import joblib
import onnx
from skl2onnx import convert_sklearn
from skl2onnx.common.data_types import FloatTensorType
import os
from pathlib import Path

# Feature names matching the detector outputs
FEATURE_NAMES = [
    'entropy_score',
    'mass_write_score',
    'mass_rename_delete_score',
    'ransom_note_score',
    'shadow_copy_score',
    'process_behavior_score',
    'file_extension_score',
    'event_rate',
    'avg_entropy_per_sec',
    'rename_delete_freq',
    'burst_interval',
    'num_detectors_firing',
    'file_diversity',
    'bytes_written_per_sec',
    'unique_extensions',
]

def generate_synthetic_data(n_samples=10000, n_benign=8000, n_malicious=2000):
    """
    Generate synthetic training data
    In production, this would load from real event logs
    """
    np.random.seed(42)
    
    # Benign process features (lower scores)
    benign_data = {
        'entropy_score': np.random.beta(2, 5, n_benign),
        'mass_write_score': np.random.beta(1, 10, n_benign),
        'mass_rename_delete_score': np.random.beta(1, 15, n_benign),
        'ransom_note_score': np.random.beta(1, 20, n_benign),
        'shadow_copy_score': np.random.beta(1, 30, n_benign),
        'process_behavior_score': np.random.beta(2, 8, n_benign),
        'file_extension_score': np.random.beta(1, 15, n_benign),
        'event_rate': np.random.gamma(2, 5, n_benign),
        'avg_entropy_per_sec': np.random.beta(2, 5, n_benign) * 4.0,
        'rename_delete_freq': np.random.gamma(1, 2, n_benign),
        'burst_interval': np.random.gamma(5, 2, n_benign),
        'num_detectors_firing': np.random.poisson(1, n_benign),
        'file_diversity': np.random.gamma(3, 2, n_benign),
        'bytes_written_per_sec': np.random.gamma(10, 1000, n_benign),
        'unique_extensions': np.random.poisson(3, n_benign),
    }
    
    # Malicious process features (higher scores)
    malicious_data = {
        'entropy_score': np.random.beta(8, 2, n_malicious),
        'mass_write_score': np.random.beta(9, 1, n_malicious),
        'mass_rename_delete_score': np.random.beta(8, 2, n_malicious),
        'ransom_note_score': np.random.beta(7, 3, n_malicious),
        'shadow_copy_score': np.random.beta(6, 4, n_malicious),
        'process_behavior_score': np.random.beta(8, 2, n_malicious),
        'file_extension_score': np.random.beta(9, 1, n_malicious),
        'event_rate': np.random.gamma(20, 2, n_malicious),
        'avg_entropy_per_sec': np.random.beta(8, 2, n_malicious) * 7.0,
        'rename_delete_freq': np.random.gamma(15, 1, n_malicious),
        'burst_interval': np.random.gamma(1, 1, n_malicious),
        'num_detectors_firing': np.random.poisson(5, n_malicious),
        'file_diversity': np.random.gamma(50, 1, n_malicious),
        'bytes_written_per_sec': np.random.gamma(100, 5000, n_malicious),
        'unique_extensions': np.random.poisson(1, n_malicious),
    }
    
    df_benign = pd.DataFrame(benign_data)
    df_benign['label'] = 0
    
    df_malicious = pd.DataFrame(malicious_data)
    df_malicious['label'] = 1
    
    df = pd.concat([df_benign, df_malicious], ignore_index=True)
    df = df.sample(frac=1, random_state=42).reset_index(drop=True)
    
    return df

def train_random_forest(X_train, y_train, X_test, y_test):
    """Train Random Forest model"""
    print("Training Random Forest...")
    rf = RandomForestClassifier(
        n_estimators=200,
        max_depth=15,
        min_samples_split=5,
        min_samples_leaf=2,
        random_state=42,
        n_jobs=-1,
        class_weight='balanced'
    )
    
    rf.fit(X_train, y_train)
    
    y_pred = rf.predict(X_test)
    y_pred_proba = rf.predict_proba(X_test)[:, 1]
    
    print("\nRandom Forest Results:")
    print(classification_report(y_test, y_pred))
    print(f"\nROC-AUC: {roc_auc_score(y_test, y_pred_proba):.4f}")
    
    return rf

def train_lightgbm(X_train, y_train, X_test, y_test):
    """Train LightGBM model"""
    print("\nTraining LightGBM...")
    
    train_data = lgb.Dataset(X_train, label=y_train)
    test_data = lgb.Dataset(X_test, label=y_test, reference=train_data)
    
    params = {
        'objective': 'binary',
        'metric': 'binary_logloss',
        'boosting_type': 'gbdt',
        'num_leaves': 31,
        'learning_rate': 0.05,
        'feature_fraction': 0.9,
        'bagging_fraction': 0.8,
        'bagging_freq': 5,
        'verbose': 0,
        'is_unbalance': True,
    }
    
    model = lgb.train(
        params,
        train_data,
        num_boost_round=200,
        valid_sets=[test_data],
        callbacks=[lgb.early_stopping(stopping_rounds=20), lgb.log_evaluation(period=10)]
    )
    
    y_pred = model.predict(X_test, num_iteration=model.best_iteration)
    y_pred_binary = (y_pred > 0.5).astype(int)
    
    print("\nLightGBM Results:")
    print(classification_report(y_test, y_pred_binary))
    print(f"\nROC-AUC: {roc_auc_score(y_test, y_pred):.4f}")
    
    return model

def export_to_onnx(model, feature_names, output_path):
    """Export scikit-learn model to ONNX"""
    print(f"\nExporting model to ONNX: {output_path}")
    
    # Define input type
    initial_type = [('float_input', FloatTensorType([None, len(feature_names)]))]
    
    # Convert to ONNX
    onnx_model = convert_sklearn(
        model,
        initial_types=initial_type,
        target_opset=13
    )
    
    # Save ONNX model
    with open(output_path, 'wb') as f:
        f.write(onnx_model.SerializeToString())
    
    print(f"ONNX model saved to {output_path}")
    
    # Verify ONNX model
    onnx_model_check = onnx.load(output_path)
    onnx.checker.check_model(onnx_model_check)
    print("ONNX model validation: PASSED")

def main():
    print("SentinelGuard ML Model Training")
    print("=" * 50)
    
    # Create output directory
    output_dir = Path("models")
    output_dir.mkdir(exist_ok=True)
    
    # Generate or load training data
    print("\nGenerating synthetic training data...")
    df = generate_synthetic_data(n_samples=10000)
    
    # Prepare features and labels
    feature_cols = FEATURE_NAMES
    X = df[feature_cols].values
    y = df['label'].values
    
    # Split data
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42, stratify=y
    )
    
    # Scale features
    scaler = StandardScaler()
    X_train_scaled = scaler.fit_transform(X_train)
    X_test_scaled = scaler.transform(X_test)
    
    # Save scaler
    scaler_path = output_dir / "scaler.joblib"
    joblib.dump(scaler, scaler_path)
    print(f"Scaler saved to {scaler_path}")
    
    # Train Random Forest
    rf_model = train_random_forest(X_train_scaled, y_train, X_test_scaled, y_test)
    
    # Save Random Forest model
    rf_path = output_dir / "random_forest.joblib"
    joblib.dump(rf_model, rf_path)
    print(f"\nRandom Forest model saved to {rf_path}")
    
    # Export to ONNX
    onnx_path = output_dir / "sentinelguard_model.onnx"
    export_to_onnx(rf_model, feature_cols, onnx_path)
    
    # Also train LightGBM (optional, for comparison)
    try:
        lgb_model = train_lightgbm(X_train, y_train, X_test, y_test)
        lgb_path = output_dir / "lightgbm.txt"
        lgb_model.save_model(str(lgb_path))
        print(f"\nLightGBM model saved to {lgb_path}")
    except Exception as e:
        print(f"\nLightGBM training skipped: {e}")
    
    print("\n" + "=" * 50)
    print("Training complete!")
    print(f"Models saved to: {output_dir.absolute()}")

if __name__ == "__main__":
    main()

