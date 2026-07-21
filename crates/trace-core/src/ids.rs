//! Identifier generation. Random UUID v4 strings keep IDs URL-safe and unique
//! across machines without a central allocator.

use uuid::Uuid;

/// Generate a new lowercase hyphenated UUID string.
pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// A short, non-cryptographic content hash (hex of a 64-bit hash). Used to
/// fingerprint a compressed prompt so history can be deduplicated/referenced
/// without storing the raw text when prompt history is disabled.
pub fn short_hash(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
