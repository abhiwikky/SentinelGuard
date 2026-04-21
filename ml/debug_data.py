"""Quick check: what does the training data look like near the decision boundary?"""
import numpy as np
import sys
sys.path.insert(0, '.')
from train import generate_synthetic_data
from features import FEATURE_NAMES

X, y = generate_synthetic_data(n_samples=50000)
ransomware = X[y == 1]
benign = X[y == 0]

# The simulation vector: [0.0, 1.0, 1.0, 1.0, 0.9, 0.37, 0.0]
# Key question: are there ransomware training samples with this profile?

# Check ransomware with: low entropy, high mass_write, high mass_rename 
simlike_ransom = ransomware[
    (ransomware[:, 0] < 0.15) &   # low entropy
    (ransomware[:, 1] > 0.7) &    # high mass_write
    (ransomware[:, 2] > 0.7)      # high mass_rename
]
print(f"Ransomware with low-entropy + high mass_write + high mass_rename: {len(simlike_ransom)}")
if len(simlike_ransom) > 0:
    print(f"  Mean profile: {simlike_ransom.mean(axis=0).round(3)}")
    # Check how many also have ransom_note > 0.5
    with_ransom = simlike_ransom[simlike_ransom[:, 3] > 0.5]
    print(f"  ...also with ransom_note > 0.5: {len(with_ransom)}")
    with_shadow = simlike_ransom[simlike_ransom[:, 4] > 0.5]
    print(f"  ...also with shadow_copy > 0.5: {len(with_shadow)}")
else:
    print("  NONE FOUND — this is the problem!")

# Check if benign has ANYTHING close
simlike_benign = benign[
    (benign[:, 1] > 0.7) &
    (benign[:, 2] > 0.7)
]
print(f"\nBenign with high mass_write + high mass_rename: {len(simlike_benign)}")

# Most critically: check the ransom_note and shadow_copy distribution
print(f"\nRansomware ransom_note stats: mean={ransomware[:, 3].mean():.3f}, >0.5: {(ransomware[:, 3] > 0.5).sum()}")
print(f"Benign ransom_note stats: mean={benign[:, 3].mean():.3f}, >0.5: {(benign[:, 3] > 0.5).sum()}")
print(f"Ransomware shadow_copy stats: mean={ransomware[:, 4].mean():.3f}, >0.5: {(ransomware[:, 4] > 0.5).sum()}")
print(f"Benign shadow_copy stats: mean={benign[:, 4].mean():.3f}, >0.5: {(benign[:, 4] > 0.5).sum()}")
