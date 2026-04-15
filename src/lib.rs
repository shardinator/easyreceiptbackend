//! Core library for the EasyReceipt backend.
//!
//! Shared logic (hashing, validation, etc.) lives here so it can be unit tested with
//! `cargo test` and reused from the HTTP server in `main.rs`.
//!
//! The Axum router from `create_router` lives in `http.rs` so integration tests can exercise the
//! API without binding a TCP port.

mod http;

pub use http::create_router;

use sha2::{Digest, Sha256};

/// Stateless SHA 256 helper: feed arbitrary bytes, get a digest (raw or hex).
///
/// This is a unit struct with only associated functions, no instance state.
/// See the Rust Book chapter on defining structs for the "unit like struct" pattern.
pub struct Sha256Hash;

impl Sha256Hash {
    /// Returns the 32 byte SHA 256 digest of `input` (UTF 8 strings, `Vec<u8>`, slices, etc.).
    pub fn digest_bytes(input: impl AsRef<[u8]>) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(input.as_ref());
        // `finalize` consumes the hasher and yields a generic array; convert to `[u8; 32]`.
        hasher.finalize().into()
    }

    /// Hexadecimal form of [`Self::digest_bytes`]: 64 lowercase hex characters, no `0x` prefix.
    pub fn digest_hex(input: impl AsRef<[u8]>) -> String {
        hex::encode(Self::digest_bytes(input))
    }
}

#[cfg(test)]
mod tests {
    // Unit tests for `Sha256Hash`; run with `cargo test` from the crate root.

    use super::Sha256Hash;

    /// NIST style sanity check: hash of the empty string is fixed and well known.
    #[test]
    fn empty_input_matches_known_digest() {
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(Sha256Hash::digest_hex(""), expected);
    }

    /// Another published test vector: SHA 256("abc").
    #[test]
    fn abc_matches_known_digest() {
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(Sha256Hash::digest_hex("abc"), expected);
    }

    /// `digest_hex` must match encoding the raw bytes from `digest_bytes`.
    #[test]
    fn digest_bytes_round_trip_with_hex() {
        let data = b"hello world";
        let bytes = Sha256Hash::digest_bytes(data);
        assert_eq!(Sha256Hash::digest_hex(data), hex::encode(bytes));
    }
}
