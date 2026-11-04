//! SHA-256 as lowercase hex: the one encoder for every digest matra
//! reports or checks.
//!
//! Three callers need it: the composition root, which records the
//! input's digest in each document's provenance, and the two model
//! adapters, which verify their pinned artifacts and report a model's
//! digest. It lives in neither adapter because adapters do not import
//! each other, and in no port because it is not a contract.

use sha2::{Digest, Sha256};

/// SHA-256 of `bytes`, lowercase hex.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Lowercase hex of a digest, two characters per byte, leading zeros
/// kept, so a pin written from `sha256sum` output compares equal.
///
/// sha2 0.11 no longer formats its output with `{:x}`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(char::from(DIGITS[usize::from(b >> 4)]));
        s.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The vector is the FIPS 180-2 SHA-256 of "abc".
    #[test]
    fn hex_renders_a_digest_as_sha256sum_does() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(hex(&[0x00, 0x0f, 0xf0, 0xff]), "000ff0ff");
    }
}
