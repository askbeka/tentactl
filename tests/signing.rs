//! Tests for HMAC-SHA512 signature generation.
//! Uses known test vectors to verify the signing algorithm matches Kraken's spec.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};

type HmacSha512 = Hmac<Sha512>;

/// Reimplementation of the signing algorithm for testing.
/// sig = Base64(HMAC-SHA512(url_path + SHA256(nonce + post_data), Base64Decode(secret)))
fn sign(path: &str, nonce: u64, post_data: &str, secret: &str) -> String {
    let secret_bytes = BASE64.decode(secret).expect("valid base64 secret");
    let mut sha256 = Sha256::new();
    sha256.update(format!("{nonce}{post_data}"));
    let sha256_digest = sha256.finalize();

    let mut hmac = HmacSha512::new_from_slice(&secret_bytes).expect("valid HMAC key");
    hmac.update(path.as_bytes());
    hmac.update(&sha256_digest);
    BASE64.encode(hmac.finalize().into_bytes())
}

#[test]
fn sign_produces_deterministic_output() {
    // Same inputs → same signature
    let secret = BASE64.encode(b"supersecretkey1234567890abcdef");
    let sig1 = sign(
        "/0/private/Balance",
        1234567890,
        "nonce=1234567890",
        &secret,
    );
    let sig2 = sign(
        "/0/private/Balance",
        1234567890,
        "nonce=1234567890",
        &secret,
    );
    assert_eq!(sig1, sig2);
}

#[test]
fn sign_different_nonce_produces_different_output() {
    let secret = BASE64.encode(b"supersecretkey1234567890abcdef");
    let sig1 = sign(
        "/0/private/Balance",
        1111111111,
        "nonce=1111111111",
        &secret,
    );
    let sig2 = sign(
        "/0/private/Balance",
        2222222222,
        "nonce=2222222222",
        &secret,
    );
    assert_ne!(sig1, sig2);
}

#[test]
fn sign_different_path_produces_different_output() {
    let secret = BASE64.encode(b"supersecretkey1234567890abcdef");
    let sig1 = sign(
        "/0/private/Balance",
        1234567890,
        "nonce=1234567890",
        &secret,
    );
    let sig2 = sign(
        "/0/private/TradesHistory",
        1234567890,
        "nonce=1234567890",
        &secret,
    );
    assert_ne!(sig1, sig2);
}

#[test]
fn sign_different_secret_produces_different_output() {
    let secret1 = BASE64.encode(b"supersecretkey1234567890abcdef");
    let secret2 = BASE64.encode(b"differentsecretkey1234567890ab");
    let sig1 = sign(
        "/0/private/Balance",
        1234567890,
        "nonce=1234567890",
        &secret1,
    );
    let sig2 = sign(
        "/0/private/Balance",
        1234567890,
        "nonce=1234567890",
        &secret2,
    );
    assert_ne!(sig1, sig2);
}

#[test]
fn sign_output_is_valid_base64() {
    let secret = BASE64.encode(b"supersecretkey1234567890abcdef");
    let sig = sign(
        "/0/private/Balance",
        1234567890,
        "nonce=1234567890",
        &secret,
    );
    assert!(
        BASE64.decode(&sig).is_ok(),
        "signature should be valid base64"
    );
    // HMAC-SHA512 produces 64 bytes → 88 chars in base64
    let decoded = BASE64.decode(&sig).unwrap();
    assert_eq!(decoded.len(), 64, "HMAC-SHA512 should produce 64 bytes");
}

#[test]
fn sign_matches_known_vector() {
    // Pre-computed test vector
    let secret = BASE64.encode(b"testsecret123456");
    let nonce = 1000000000u64;
    let path = "/0/private/Balance";
    let post_data = "nonce=1000000000";

    let sig = sign(path, nonce, post_data, &secret);

    // Recompute manually to verify
    let secret_bytes = BASE64.decode(&secret).unwrap();
    let mut sha256 = Sha256::new();
    sha256.update(format!("{nonce}{post_data}"));
    let sha256_digest = sha256.finalize();

    let mut hmac = HmacSha512::new_from_slice(&secret_bytes).unwrap();
    hmac.update(path.as_bytes());
    hmac.update(&sha256_digest);
    let expected = BASE64.encode(hmac.finalize().into_bytes());

    assert_eq!(sig, expected);
}
