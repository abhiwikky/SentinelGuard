"""Diagnose the current ONNX model's outputs for known benign/ransomware inputs."""
import numpy as np
import onnxruntime as ort

MODEL_PATH = "model.onnx"

session = ort.InferenceSession(MODEL_PATH)
input_name = session.get_inputs()[0].name
output_names = [o.name for o in session.get_outputs()]

print(f"Model: {MODEL_PATH}")
print(f"Input: {input_name}, shape={session.get_inputs()[0].shape}")
print(f"Outputs: {output_names}")
print()

# Features: [entropy, mass_write, mass_rename, ransom_note, shadow_copy, process_behavior, ext_explosion]
test_cases = {
    "All zeros (idle process)":         [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    "Tiny noise (browser-like)":        [0.02, 0.01, 0.0, 0.0, 0.0, 0.03, 0.01],
    "Process behavior only (40%)":      [0.0, 0.0, 0.0, 0.0, 0.0, 0.4, 0.0],
    "Browser moderate activity":        [0.0, 0.05, 0.0, 0.0, 0.0, 0.4, 0.05],
    "Build tool (mass write+ext)":      [0.1, 0.5, 0.0, 0.0, 0.0, 0.3, 0.1],
    "** SIMULATE_ATTACKS EXACT **":     [0.0, 1.0, 1.0, 1.0, 0.9, 0.37, 0.0],
    "Early ransomware (no entropy)":    [0.0, 0.8, 0.7, 0.5, 0.0, 0.4, 0.1],
    "Full ransomware":                  [0.95, 0.9, 0.85, 0.7, 0.4, 0.8, 0.9],
    "Classic ransomware":               [0.9, 0.8, 0.7, 0.5, 0.0, 0.6, 0.8],
}

print(f"{'Test Case':<40} {'Label':>6} {'P(benign)':>10} {'P(ransom)':>10}")
print("-" * 70)

for name, features in test_cases.items():
    inp = np.array([features], dtype=np.float32)
    results = session.run(None, {input_name: inp})
    
    label = results[0][0] if len(results) > 0 else "?"
    probs = results[1][0] if len(results) > 1 else results[0][0]
    
    if hasattr(probs, '__len__') and len(probs) >= 2:
        print(f"{name:<40} {label:>6} {probs[0]:>10.4f} {probs[1]:>10.4f}")
    else:
        print(f"{name:<40} {label:>6} {'N/A':>10} {float(probs):>10.4f}")
