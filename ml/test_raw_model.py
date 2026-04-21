"""Test different model architectures on the SAME data."""
import numpy as np, sys; sys.path.insert(0, '.')
from train import generate_synthetic_data
from sklearn.model_selection import train_test_split
from sklearn.neural_network import MLPClassifier
from sklearn.ensemble import RandomForestClassifier
from lightgbm import LGBMClassifier
import warnings; warnings.filterwarnings('ignore')

X, y = generate_synthetic_data(50000)
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2, random_state=42, stratify=y)

sim = np.array([[0.0, 1.0, 1.0, 1.0, 0.9, 0.37, 0.0]], dtype=np.float32)

models = {
    "LightGBM (default)": LGBMClassifier(n_estimators=500, verbose=-1, random_state=42),
    "LightGBM (no depth limit)": LGBMClassifier(n_estimators=500, max_depth=-1, num_leaves=63, min_child_samples=5, verbose=-1, random_state=42),
    "RandomForest": RandomForestClassifier(n_estimators=300, max_depth=8, random_state=42, n_jobs=-1),
    "MLP (2 layers)": MLPClassifier(hidden_layer_sizes=(64, 32), max_iter=500, random_state=42),
    "MLP (3 layers)": MLPClassifier(hidden_layer_sizes=(128, 64, 32), max_iter=500, random_state=42),
}

print(f"{'Model':<30} {'Acc':>6} {'Sim P(ransom)':>14}")
print("-" * 54)
for name, model in models.items():
    model.fit(X_train, y_train)
    acc = model.score(X_test, y_test)
    proba = model.predict_proba(sim)[0][1]
    print(f"{name:<30} {acc:>6.4f} {proba:>14.6f}")
