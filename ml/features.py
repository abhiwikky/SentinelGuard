"""
SentinelGuard ML Training Pipeline - Feature Definitions

Defines the feature vector used by the ransomware detection model.
The feature order here MUST match the Rust agent's extract_features()
method in inference.rs.

Features (7 total):
    0: entropy_spike       - Max entropy detector score in window
    1: mass_write          - Max mass write detector score
    2: mass_rename_delete  - Max mass rename/delete detector score
    3: ransom_note         - Max ransom note detector score
    4: shadow_copy         - Max shadow copy detector score
    5: process_behavior    - Max process behavior detector score
    6: extension_explosion - Max extension explosion detector score

Each feature is a float in [0.0, 1.0].
Target: 0 = benign, 1 = ransomware
"""

FEATURE_NAMES = [
    "entropy_spike",
    "mass_write",
    "mass_rename_delete",
    "ransom_note",
    "shadow_copy",
    "process_behavior",
    "extension_explosion",
]

NUM_FEATURES = len(FEATURE_NAMES)


def validate_features(features: list[float]) -> bool:
    """Validate a feature vector."""
    if len(features) != NUM_FEATURES:
        return False
    return all(0.0 <= f <= 1.0 for f in features)


def describe_features():
    """Print feature descriptions."""
    descriptions = {
        "entropy_spike": "Shannon entropy of written files exceeding threshold",
        "mass_write": "Volume of write operations within time window",
        "mass_rename_delete": "Volume of rename/delete operations within time window",
        "ransom_note": "Detection of files matching ransom note name patterns",
        "shadow_copy": "Shadow copy deletion or suspicious system process activity",
        "process_behavior": "Anomalous breadth of file extensions and directories",
        "extension_explosion": "Creation of many previously-unseen file extensions",
    }
    for i, name in enumerate(FEATURE_NAMES):
        print(f"  [{i}] {name}: {descriptions.get(name, 'N/A')}")
