//! # httpx-crypto: AEAD-Native Framing
//!
//! ## Performance Contract
//! - **Symmetric Transform**: ~0.8 cycles/byte (ChaCha20-Poly1305).
//! - **Overhead**: 0-RTT latency (Handshake-less initialization).

use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, Tag};
use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use zeroize::Zeroizing;

/// A trait for high-performance, in-place Authenticated Encryption.
///
/// Designed to work directly within io_uring or DPDK registered buffers.
pub trait SecureInPlaceAEAD {
    /// Encrypts data directly within the provided buffer.
    fn seal_in_place(
        &self,
        key: &Zeroizing<[u8; 32]>,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, CryptoError>;

    /// Decrypts data directly within the provided buffer.
    fn open_in_place(
        &self,
        key: &Zeroizing<[u8; 32]>,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), CryptoError>;
}

#[derive(Debug)]
pub enum CryptoError {
    HandshakeFailure,
    IntegrityCheckFailed,
    KeyZeroizeError,
}

pub struct AEADStack;

impl SecureInPlaceAEAD for AEADStack {
    #[inline(always)]
    fn seal_in_place(
        &self,
        key: &Zeroizing<[u8; 32]>,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, CryptoError> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&**key));
        let nonce = Nonce::from_slice(nonce);
        
        cipher.encrypt_in_place_detached(nonce, aad, buffer)
            .map_err(|_| CryptoError::IntegrityCheckFailed)
    }

    #[inline(always)]
    fn open_in_place(
        &self,
        key: &Zeroizing<[u8; 32]>,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), CryptoError> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&**key));
        let nonce = Nonce::from_slice(nonce);

        cipher.decrypt_in_place_detached(nonce, aad, buffer, tag)
            .map_err(|_| CryptoError::IntegrityCheckFailed)
    }
}
