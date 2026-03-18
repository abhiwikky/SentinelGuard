//! SentinelGuard Security Module
//!
//! Provides security utilities: hashing, integrity checks, and
//! privilege verification.

use anyhow::Result;
use sha2::{Digest, Sha256};
use tracing::warn;

/// Verify the running process has administrator privileges
pub fn verify_admin_privileges() -> bool {
    // On Windows, check if running as admin using a simple approach:
    // Try to read a protected system location
    use std::fs;
    let admin_path = r"C:\Windows\System32\config";
    fs::read_dir(admin_path).is_ok()
}

/// Compute SHA-256 hash of file contents
pub fn hash_file(path: &str) -> Result<String> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Compute SHA-256 hash of byte data
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Validate that a binary at the given path matches an expected hash.
/// Returns true if the hash matches or if validation is skipped.
pub fn validate_binary_integrity(path: &str, expected_hash: Option<&str>) -> bool {
    if let Some(expected) = expected_hash {
        match hash_file(path) {
            Ok(actual) => {
                if actual != expected {
                    warn!(
                        "Binary integrity check failed for {}: expected={}, actual={}",
                        path, expected, actual
                    );
                    false
                } else {
                    true
                }
            }
            Err(e) => {
                warn!("Failed to hash binary {}: {}", path, e);
                false
            }
        }
    } else {
        // No expected hash provided, skip validation
        true
    }
}

/// Verify the gRPC listen address is localhost-only
pub fn is_localhost_address(addr: &str) -> bool {
    addr.starts_with("127.0.0.1:")
        || addr.starts_with("localhost:")
        || addr.starts_with("[::1]:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_bytes() {
        let hash = hash_bytes(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_localhost_check() {
        assert!(is_localhost_address("127.0.0.1:50051"));
        assert!(is_localhost_address("localhost:3000"));
        assert!(is_localhost_address("[::1]:50051"));
        assert!(!is_localhost_address("0.0.0.0:50051"));
        assert!(!is_localhost_address("192.168.1.1:50051"));
    }
}
