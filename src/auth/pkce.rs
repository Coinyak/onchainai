//! PKCE code verifier/challenge for Supabase OAuth (S256).

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use getrandom::getrandom;
use sha2::{Digest, Sha256};

const VERIFIER_LEN: usize = 64;

/// Generate a random PKCE verifier and its S256 challenge (base64url, no padding).
pub fn generate_pkce_pair() -> (String, String) {
    let mut bytes = [0u8; VERIFIER_LEN];
    getrandom(&mut bytes).expect("OS random unavailable");
    let verifier = URL_SAFE_NO_PAD.encode(bytes);
    let challenge = challenge_s256(&verifier);
    (verifier, challenge)
}

/// SHA256(verifier) as base64url without padding.
pub fn challenge_s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_pair_is_deterministic_challenge() {
        let (verifier, challenge) = generate_pkce_pair();
        assert!(!verifier.is_empty());
        assert_eq!(challenge, challenge_s256(&verifier));
    }
}
