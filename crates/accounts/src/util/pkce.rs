//! The proof a PKCE sign-in carries from its start to its token exchange.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// `bytes` of randomness, URL-safe.
pub fn random_token(bytes: usize) -> Result<String, getrandom::Error> {
    let mut buffer = vec![0; bytes];
    getrandom::fill(&mut buffer)?;
    Ok(URL_SAFE_NO_PAD.encode(buffer))
}

/// The `S256` challenge for `verifier`.
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_matches_the_rfc_example() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn a_verifier_is_long_enough_and_url_safe() {
        let token = random_token(32).expect("randomness");
        assert_eq!(token.len(), 43);
        assert!(
            token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        );
    }
}
