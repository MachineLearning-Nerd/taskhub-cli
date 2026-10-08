//! API tokens: the secret wrapper and the local format check, matching TaskHub's
//! `lib/auth/api-token-format.ts` (`thk_` + 43 base64url characters + 6-character base62 CRC-32).

use crate::error::{CliError, Code, Result};
use std::fmt;

const PREFIX: &str = "thk_";
const PAYLOAD_LEN: usize = 43;
const CHECKSUM_LEN: usize = 6;
const BASE62: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// A token. It cannot be printed: `Debug` hides it and there is no `Display`.
/// The value leaves this type only through `expose_for_header` (the HTTP client) and `expose_for_storage`
/// (the credentials file).
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Accepts only a well-formed TaskHub token, after trimming surrounding whitespace (a trailing newline
    /// from `echo` or a file is common).
    pub fn parse(raw: &str) -> Result<Secret> {
        let value = raw.trim();
        if !is_api_token(value) {
            return Err(CliError::new(Code::InvalidInput, "That is not a TaskHub API token.").with_hint(
                "Copy the whole token, starting with thk_, from Settings → API tokens in TaskHub.",
            ));
        }
        Ok(Secret(value.to_owned()))
    }

    pub fn expose_for_header(&self) -> &str {
        &self.0
    }

    pub fn expose_for_storage(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(…)")
    }
}

pub fn is_api_token(value: &str) -> bool {
    let Some(rest) = value.strip_prefix(PREFIX) else { return false };
    if rest.len() != PAYLOAD_LEN + CHECKSUM_LEN || !rest.is_ascii() {
        return false;
    }
    let (payload, checksum) = rest.split_at(PAYLOAD_LEN);
    let payload_ok = payload.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    payload_ok && checksum.bytes().all(|b| b.is_ascii_alphanumeric()) && token_checksum(payload) == checksum
}

/// CRC-32/ISO-HDLC (reflected polynomial 0xedb88320, init and final XOR 0xffffffff), base62, zero-padded to 6.
pub fn token_checksum(payload: &str) -> String {
    let mut value = crc32(payload.as_bytes());
    let mut digits = Vec::with_capacity(CHECKSUM_LEN);
    loop {
        digits.push(BASE62[(value % 62) as usize]);
        value /= 62;
        if value == 0 {
            break;
        }
    }
    while digits.len() < CHECKSUM_LEN {
        digits.push(b'0');
    }
    digits.reverse();
    String::from_utf8(digits).unwrap_or_default()
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    crc ^ 0xffff_ffff
}

#[cfg(test)]
pub(crate) fn test_token(payload_char: char) -> String {
    let payload: String = std::iter::repeat_n(payload_char, PAYLOAD_LEN).collect();
    format!("{PREFIX}{payload}{}", token_checksum(&payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vectors computed with TaskHub's own tokenChecksum.
    #[test]
    fn checksum_matches_taskhub_vectors() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(token_checksum("123456789"), "3jZRME");
        assert_eq!(token_checksum(""), "000000");
        assert_eq!(token_checksum(&"A".repeat(43)), "0DofJ8");
        assert_eq!(token_checksum("abcdefghijklmnopqrstuvwxyz0123456789-_ABCDE"), "3tEw51");
    }

    #[test]
    fn accepts_valid_tokens_and_refuses_malformed_ones() {
        let valid = format!("thk_{}0DofJ8", "A".repeat(43));
        assert!(is_api_token(&valid));
        assert!(Secret::parse(&format!("{valid}\n")).is_ok());
        for bad in [
            String::new(),
            "thk_".into(),
            format!("THK_{}", "a".repeat(49)),
            format!("thk_{}", "a".repeat(48)),
            format!("thk_{}", "!".repeat(49)),
            format!("thk_{}0DofJ9", "A".repeat(43)),
            format!("thk_B{}0DofJ8", "A".repeat(42)),
        ] {
            assert!(!is_api_token(&bad), "{bad:?}");
        }
    }

    #[test]
    fn secrets_never_print() {
        let secret = Secret::parse(&test_token('Z')).unwrap();
        assert_eq!(format!("{secret:?}"), "Secret(…)");
    }
}
