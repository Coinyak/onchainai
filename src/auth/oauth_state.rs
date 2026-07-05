//! HMAC-signed OAuth `state` parameters — CSRF protection without relying solely on cookies.
//!
//! Browsers (especially Safari) and some reverse proxies can drop the HttpOnly state cookie
//! across the GitHub/Google redirect round-trip. The signed `state` query param still
//! validates when the cookie is missing.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use getrandom::getrandom;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

fn sign_payload(secret: &str, payload: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

/// Mint a URL-safe OAuth state value: `{nonce}.{hmac}`.
pub fn mint_oauth_state(secret: &str) -> String {
    let mut nonce = [0u8; 16];
    getrandom(&mut nonce).expect("OS random unavailable");
    let payload = URL_SAFE_NO_PAD.encode(nonce);
    let mac = sign_payload(secret, &payload);
    format!("{payload}.{mac}")
}

/// Verify a minted OAuth state (constant-time MAC compare).
pub fn verify_oauth_state(secret: &str, state: &str) -> bool {
    let Some((payload, mac)) = state.rsplit_once('.') else {
        return false;
    };
    if payload.is_empty() || mac.is_empty() {
        return false;
    }
    let expected = sign_payload(secret, payload);
    expected.len() == mac.len()
        && expected
            .bytes()
            .zip(mac.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_and_verify_round_trip() {
        let secret = "test-jwt-secret";
        let state = mint_oauth_state(secret);
        assert!(verify_oauth_state(secret, &state));
    }

    #[test]
    fn verify_rejects_tampered_state() {
        let state = mint_oauth_state("secret-a");
        assert!(!verify_oauth_state("secret-b", &state));
        assert!(!verify_oauth_state("secret-a", "not-a-valid-state"));
    }
}