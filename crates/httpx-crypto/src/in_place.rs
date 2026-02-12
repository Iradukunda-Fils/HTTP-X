use httpx_core::error::HttpXError;
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chacha20poly1305::aead::{AeadInPlace, KeyInit, Tag};

/// High-Performance In-Place Secure Transformation.
/// 
/// This trait uses STATIC DISPATCH to ensure the compiler can inline
/// the cryptographic primitives directly into the transport loop.
pub trait SecureTransformer {
    /// Performs in-place transformation within the io_uring registered buffer.
    /// ZERO heap allocations. ZERO memory copies.
    fn transform_in_place<A: AeadInPlace>(
        &self,
        aead: &A,
        nonce: &Nonce,
        associated_data: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), HttpXError>;
}

pub struct HardwareAlignedCrypto;

impl SecureTransformer for HardwareAlignedCrypto {
    #[inline(always)]
    fn transform_in_place<A: AeadInPlace>(
        &self,
        aead: &A,
        nonce: &Nonce,
        associated_data: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), HttpXError> {
        // SAFETY: The buffer is guaranteed to be within the io_uring registered region
        // and its bounds are checked at the frame boundary before this call.
        aead.decrypt_in_place_detached(nonce, associated_data, buffer, tag)
            .map_err(|_| HttpXError::ProtocolViolation("AEAD Integrity Failure".to_string()))
    }
}
