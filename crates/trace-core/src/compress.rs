//! Trace Compression — local, lossless compression of the bulky text Trace
//! stores per run (chiefly the captured diff).
//!
//! Design goals, mapped to the product requirements:
//!
//! * **Real, not cosmetic.** Data is actually run through gzip (DEFLATE) and
//!   stored compressed; reading transparently decompresses. There is no hosted
//!   service and no API key — it is 100% local and deterministic.
//! * **Correctness-preserving.** Compression is lossless: `decompress(compress(x))
//!   == x` for all inputs (tested). The information Trace needs is never altered.
//! * **Connector-agnostic.** It operates at the *storage boundary* on the final
//!   captured artifact, so it behaves identically no matter which agent
//!   (Claude Code, Cursor, Codex, …) produced the run.
//! * **Safe fallback.** [`decode`] auto-detects whether stored bytes are gzip or
//!   legacy plaintext (by the gzip magic number), so pre-compression data still
//!   reads back correctly and a connector that stored plaintext never breaks.
//! * **Never inflates.** [`encode`] only keeps the compressed form when it is
//!   actually smaller; tiny inputs are stored as-is.
//!
//! NOTE ON SCOPE: this is *storage* compression. A separate, optional
//! *query-aware token-reduction* path (shrinking an LLM prompt/context before it
//! reaches a model) would require an external provider and API key — see
//! `docs`/the CLI help — and is intentionally not wired in here, so nothing
//! silently depends on a credential that isn't configured.

use std::io::{Read, Write};

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Serialize;

/// The gzip magic number (RFC 1952). Used to tell compressed bytes from legacy
/// plaintext when decoding.
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// Measured effect of compressing a payload. All sizes are in bytes.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct CompressionStats {
    pub original_bytes: usize,
    pub compressed_bytes: usize,
}

impl CompressionStats {
    /// Fraction of size removed, 0.0–1.0 (0 when the input was empty or grew).
    pub fn reduction(&self) -> f64 {
        if self.original_bytes == 0 || self.compressed_bytes >= self.original_bytes {
            return 0.0;
        }
        1.0 - (self.compressed_bytes as f64 / self.original_bytes as f64)
    }

    /// Bytes saved (never negative — [`encode`] never stores an inflated form).
    pub fn saved_bytes(&self) -> usize {
        self.original_bytes.saturating_sub(self.compressed_bytes)
    }
}

/// gzip-compress raw bytes. Lossless.
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).context("compressing")?;
    enc.finish().context("finalizing compression")
}

/// gzip-decompress bytes produced by [`compress`]. Lossless.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    GzDecoder::new(data)
        .read_to_end(&mut out)
        .context("decompressing")?;
    Ok(out)
}

/// True when `bytes` look like a gzip stream (start with the magic number).
pub fn is_compressed(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0..2] == GZIP_MAGIC
}

/// Encode text for storage: gzip it, but keep whichever form is smaller so a
/// tiny payload is never inflated by the gzip header. Returns the bytes to
/// store plus the measured stats. The returned bytes are self-describing —
/// [`decode`] reads either form back.
pub fn encode(text: &str) -> (Vec<u8>, CompressionStats) {
    let original = text.as_bytes();
    let original_bytes = original.len();
    match compress(original) {
        Ok(gz) if gz.len() < original_bytes => {
            let compressed_bytes = gz.len();
            (
                gz,
                CompressionStats {
                    original_bytes,
                    compressed_bytes,
                },
            )
        }
        // Compression didn't help (or errored): store plaintext.
        _ => (
            original.to_vec(),
            CompressionStats {
                original_bytes,
                compressed_bytes: original_bytes,
            },
        ),
    }
}

/// Decode stored bytes back to text, auto-detecting gzip vs. legacy plaintext.
/// This is what makes the change backward-compatible: rows written before
/// compression existed (plain UTF-8) still read correctly.
pub fn decode(bytes: &[u8]) -> Result<String> {
    let raw = if is_compressed(bytes) {
        decompress(bytes)?
    } else {
        bytes.to_vec()
    };
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_is_lossless_for_varied_inputs() {
        for s in [
            "",
            "a",
            "diff --git a/x b/x\n+const k = 1;\n-const k = 0;\n",
            &"repeated line\n".repeat(500),
            "unicode: café ☕ 日本語 — emoji 🎉\n",
        ] {
            let (bytes, _stats) = encode(s);
            assert_eq!(decode(&bytes).unwrap(), *s, "round trip failed for {s:?}");
        }
    }

    #[test]
    fn compresses_repetitive_text_measurably() {
        let big = "the quick brown fox jumps over the lazy dog\n".repeat(1000);
        let (bytes, stats) = encode(&big);
        assert!(
            is_compressed(&bytes),
            "large repetitive text should compress"
        );
        assert!(stats.compressed_bytes < stats.original_bytes);
        assert!(
            stats.reduction() > 0.9,
            "reduction was {}",
            stats.reduction()
        );
        assert_eq!(decode(&bytes).unwrap(), big);
    }

    #[test]
    fn tiny_input_is_not_inflated() {
        let (bytes, stats) = encode("hi");
        // Stored form is never larger than the original.
        assert!(stats.compressed_bytes <= stats.original_bytes);
        assert!(!is_compressed(&bytes)); // stored as plaintext
        assert_eq!(decode(&bytes).unwrap(), "hi");
    }

    #[test]
    fn legacy_plaintext_still_decodes() {
        // Simulate a pre-compression row: raw UTF-8 bytes, no gzip header.
        let legacy = "old plaintext diff\n+line\n".as_bytes();
        assert!(!is_compressed(legacy));
        assert_eq!(decode(legacy).unwrap(), "old plaintext diff\n+line\n");
    }
}
