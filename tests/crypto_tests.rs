//! # Crypto Layer Tests: AEAD In-Place Transformation
//!
//! Validates ChaCha20-Poly1305 encrypt/decrypt roundtrip
//! using the crate's `SecureInPlaceAEAD` trait and `AEADStack`.

use httpx_crypto::{SecureInPlaceAEAD, AEADStack};
use zeroize::Zeroizing;
use std::time::Instant;

/// Verifies successful in-place encrypt → decrypt roundtrip.
#[test]
fn test_aead_decrypt_valid() {
    let t = Instant::now();

    let key = Zeroizing::new(*b"an example very very secret key.");
    let nonce = b"unique nonce";
    let aad = b"associated-data";

    let plaintext = b"Hello, HTTP-X Sovereign World!!";
    let mut buffer = plaintext.to_vec();

    let stack = AEADStack;

    // Encrypt
    let tag = stack.seal_in_place(&key, nonce, aad, &mut buffer)
        .expect("Encryption failed");

    // Decrypt
    let result = stack.open_in_place(&key, nonce, aad, &mut buffer, &tag);
    assert!(result.is_ok(), "Decryption should succeed with valid data");
    assert_eq!(&buffer, plaintext, "Decrypted data should match original plaintext");

    let overhead = t.elapsed();
    println!("test_aead_decrypt_valid: Testing Overhead = {:?}", overhead);
}

/// Verifies that tampered ciphertext returns an error.
#[test]
fn test_aead_decrypt_tampered() {
    let t = Instant::now();

    let key = Zeroizing::new(*b"an example very very secret key.");
    let nonce = b"unique nonce";
    let aad = b"associated-data";

    let plaintext = b"Hello, HTTP-X Sovereign World!!";
    let mut buffer = plaintext.to_vec();

    let stack = AEADStack;

    let tag = stack.seal_in_place(&key, nonce, aad, &mut buffer)
        .expect("Encryption failed");

    // Tamper with the ciphertext
    buffer[0] ^= 0xFF;

    let result = stack.open_in_place(&key, nonce, aad, &mut buffer, &tag);
    assert!(result.is_err(), "Decryption should fail with tampered data");

    let overhead = t.elapsed();
    println!("test_aead_decrypt_tampered: Testing Overhead = {:?}", overhead);
}
