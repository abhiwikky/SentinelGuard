"""
SentinelGuard E2E Simulation

Simulates ransomware-like file system activity to test the detection pipeline.
This is a SIMULATION - it creates temporary files and performs rapid operations
that mimic ransomware behavior patterns.

Usage:
    python e2e_sim.py [--target-dir C:\\Temp\\sg_test] [--intensity medium]

WARNING: This creates and modifies files rapidly. Only run in a dedicated
test directory. Never run against production data.
"""

import argparse
import os
import random
import string
import time
import json
import sys

def random_string(length: int = 16) -> str:
    return ''.join(random.choices(string.ascii_letters + string.digits, k=length))

def create_test_files(target_dir: str, count: int = 50) -> list[str]:
    """Create a set of test files with random content."""
    os.makedirs(target_dir, exist_ok=True)
    files = []
    extensions = ['.txt', '.doc', '.pdf', '.jpg', '.png', '.xlsx', '.pptx',
                  '.csv', '.html', '.xml']

    for i in range(count):
        ext = random.choice(extensions)
        filename = f"testfile_{i:04d}{ext}"
        filepath = os.path.join(target_dir, filename)

        # Write some recognizable content
        content = f"SentinelGuard test file {i}\n" + random_string(random.randint(100, 5000))
        with open(filepath, 'w') as f:
            f.write(content)

        files.append(filepath)

    print(f"  Created {count} test files in {target_dir}")
    return files


def simulate_ransomware_encryption(files: list[str], target_dir: str):
    """
    Simulate ransomware encryption behavior:
    1. Read each file
    2. Write encrypted (high-entropy) content
    3. Rename with a custom extension
    4. Create ransom note
    """
    encrypted_ext = f".{random_string(4).lower()}"
    print(f"  Simulating encryption with extension: {encrypted_ext}")

    for filepath in files:
        try:
            # Read original
            with open(filepath, 'rb') as f:
                original = f.read()

            # Write "encrypted" (random high-entropy) content
            encrypted = bytes(random.getrandbits(8) for _ in range(len(original)))
            with open(filepath, 'wb') as f:
                f.write(encrypted)

            # Rename with new extension
            new_path = filepath + encrypted_ext
            os.rename(filepath, new_path)

        except (OSError, IOError) as e:
            print(f"    Warning: Could not process {filepath}: {e}")

        # Small delay to make it realistic but detectable
        time.sleep(0.01)

    # Create ransom note
    note_path = os.path.join(target_dir, "README.txt")
    with open(note_path, 'w') as f:
        f.write("YOUR FILES HAVE BEEN ENCRYPTED\n")
        f.write("This is a SentinelGuard simulation.\n")
        f.write("No actual encryption occurred.\n")

    print(f"  Ransom note created: {note_path}")


def simulate_mass_delete(target_dir: str, count: int = 30):
    """Simulate mass file deletion."""
    print(f"  Simulating mass deletion of {count} files...")
    files = []
    for i in range(count):
        filepath = os.path.join(target_dir, f"delete_target_{i}.tmp")
        with open(filepath, 'w') as f:
            f.write(random_string(100))
        files.append(filepath)

    for filepath in files:
        try:
            os.remove(filepath)
        except OSError:
            pass
        time.sleep(0.005)


def simulate_extension_explosion(target_dir: str):
    """Simulate creating files with many unusual extensions."""
    print("  Simulating extension explosion...")
    extensions = ['.locked', '.encrypted', '.enc', '.crypt', '.aaa', '.bbb',
                  '.zzz', '.pays', '.ransom', '.vault', '.cerber', '.locky',
                  '.zepto', '.thor', '.micro', '.crypted']

    for ext in extensions:
        filepath = os.path.join(target_dir, f"explosion_test{ext}")
        with open(filepath, 'w') as f:
            f.write(random_string(200))
        time.sleep(0.01)


def cleanup(target_dir: str):
    """Remove all test files."""
    import shutil
    if os.path.exists(target_dir):
        shutil.rmtree(target_dir)
        print(f"  Cleaned up: {target_dir}")


def main():
    parser = argparse.ArgumentParser(description="SentinelGuard E2E Simulation")
    parser.add_argument("--target-dir", default=os.path.join(os.environ.get('TEMP', 'C:\\Temp'), 'sg_e2e_test'),
                        help="Directory for test files")
    parser.add_argument("--intensity", choices=["low", "medium", "high"], default="medium",
                        help="Simulation intensity")
    parser.add_argument("--no-cleanup", action="store_true",
                        help="Don't clean up test files after simulation")
    args = parser.parse_args()

    file_counts = {"low": 20, "medium": 50, "high": 200}
    count = file_counts[args.intensity]

    print(f"\nSentinelGuard E2E Simulation")
    print(f"{'=' * 50}")
    print(f"  Target: {args.target_dir}")
    print(f"  Intensity: {args.intensity} ({count} files)")
    print()

    # Phase 1: Create test files
    print("[Phase 1] Creating test files...")
    files = create_test_files(args.target_dir, count)

    # Phase 2: Simulate encryption
    print("\n[Phase 2] Simulating ransomware encryption...")
    simulate_ransomware_encryption(files, args.target_dir)

    # Phase 3: Mass deletion
    print("\n[Phase 3] Simulating mass deletion...")
    simulate_mass_delete(args.target_dir, count // 2)

    # Phase 4: Extension explosion
    print("\n[Phase 4] Simulating extension explosion...")
    simulate_extension_explosion(args.target_dir)

    print(f"\n{'=' * 50}")
    print("Simulation complete.")
    print("Check the SentinelGuard dashboard for detection results.")
    print()

    if not args.no_cleanup:
        print("[Cleanup] Removing test files...")
        cleanup(args.target_dir)

    print("Done.")


if __name__ == "__main__":
    main()
