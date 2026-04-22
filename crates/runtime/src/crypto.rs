//! Cryptographic operations for Seq
//!
//! These functions are exported with C ABI for LLVM codegen to call.
//!
//! # API
//!
//! ```seq
//! # SHA-256 hashing
//! "hello" crypto.sha256                    # ( String -- String ) hex digest
//!
//! # HMAC-SHA256 for webhook verification
//! "message" "secret" crypto.hmac-sha256    # ( String String -- String ) hex signature
//!
//! # Timing-safe comparison
//! sig1 sig2 crypto.constant-time-eq        # ( String String -- Bool )
//!
//! # Secure random bytes
//! 32 crypto.random-bytes                   # ( Int -- String ) hex-encoded random bytes
//!
//! # UUID v4
//! crypto.uuid4                             # ( -- String ) "550e8400-e29b-41d4-a716-446655440000"
//!
//! # AES-256-GCM authenticated encryption
//! plaintext hex-key crypto.aes-gcm-encrypt  # ( String String -- String Bool )
//! ciphertext hex-key crypto.aes-gcm-decrypt # ( String String -- String Bool )
//!
//! # Key derivation from password
//! password salt iterations crypto.pbkdf2-sha256  # ( String String Int -- String Bool )
//! ```

use crate::seqstring::global_string;
use crate::stack::{Stack, pop, push};
use crate::value::Value;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit as AesKeyInit, OsRng, rand_core::RngCore as AeadRngCore},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hmac::{Hmac, Mac};
use rand::{RngCore, rng};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

const AES_NONCE_SIZE: usize = 12;
const AES_KEY_SIZE: usize = 32;
const AES_GCM_TAG_SIZE: usize = 16;
const MIN_PBKDF2_ITERATIONS: i64 = 1_000;

type HmacSha256 = Hmac<Sha256>;

/// Compute SHA-256 hash of a string
///
/// Stack effect: ( String -- String )
///
/// Returns the hash as a lowercase hex string (64 characters).
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_sha256(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "sha256: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::String(s) => {
            let mut hasher = Sha256::new();
            hasher.update(s.as_str().as_bytes());
            let result = hasher.finalize();
            let hex_digest = hex::encode(result);
            unsafe { push(stack, Value::String(global_string(hex_digest))) }
        }
        _ => panic!("sha256: expected String on stack, got {:?}", value),
    }
}

/// Compute HMAC-SHA256 of a message with a key
///
/// Stack effect: ( message key -- String )
///
/// Returns the signature as a lowercase hex string (64 characters).
/// Used for webhook verification, JWT signing, API authentication.
///
/// # Safety
/// Stack must have two String values on top (message, then key)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_hmac_sha256(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "hmac-sha256: stack is empty");

    let (stack, key_value) = unsafe { pop(stack) };
    let (stack, msg_value) = unsafe { pop(stack) };

    match (msg_value, key_value) {
        (Value::String(msg), Value::String(key)) => {
            let mut mac = <HmacSha256 as Mac>::new_from_slice(key.as_str().as_bytes())
                .expect("HMAC can take any key");
            mac.update(msg.as_str().as_bytes());
            let result = mac.finalize();
            let hex_sig = hex::encode(result.into_bytes());
            unsafe { push(stack, Value::String(global_string(hex_sig))) }
        }
        (msg, key) => panic!(
            "hmac-sha256: expected (String, String) on stack, got ({:?}, {:?})",
            msg, key
        ),
    }
}

/// Timing-safe string comparison
///
/// Stack effect: ( String String -- Bool )
///
/// Compares two strings in constant time to prevent timing attacks.
/// Essential for comparing signatures, hashes, tokens, etc.
///
/// Uses the `subtle` crate for cryptographically secure constant-time comparison.
/// This prevents timing side-channel attacks where an attacker could deduce
/// secret values by measuring comparison duration.
///
/// # Safety
/// Stack must have two String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_constant_time_eq(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "constant-time-eq: stack is empty");

    let (stack, b_value) = unsafe { pop(stack) };
    let (stack, a_value) = unsafe { pop(stack) };

    match (a_value, b_value) {
        (Value::String(a), Value::String(b)) => {
            let a_bytes = a.as_str().as_bytes();
            let b_bytes = b.as_str().as_bytes();

            // Use subtle crate for truly constant-time comparison
            // This handles different-length strings correctly without timing leaks
            let eq = a_bytes.ct_eq(b_bytes);

            unsafe { push(stack, Value::Bool(bool::from(eq))) }
        }
        (a, b) => panic!(
            "constant-time-eq: expected (String, String) on stack, got ({:?}, {:?})",
            a, b
        ),
    }
}

/// Generate cryptographically secure random bytes
///
/// Stack effect: ( Int -- String )
///
/// Returns the random bytes as a lowercase hex string (2 chars per byte).
/// Uses the operating system's secure random number generator.
///
/// # Limits
/// - Maximum: 1024 bytes (to prevent memory exhaustion)
/// - Common use cases: 16-32 bytes for tokens/nonces, 32-64 bytes for keys
///
/// # Safety
/// Stack must have an Int value on top (number of bytes to generate)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_random_bytes(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "random-bytes: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::Int(n) => {
            if n < 0 {
                panic!("random-bytes: byte count must be non-negative, got {}", n);
            }
            if n > 1024 {
                panic!("random-bytes: byte count too large (max 1024), got {}", n);
            }

            let mut bytes = vec![0u8; n as usize];
            rng().fill_bytes(&mut bytes);
            let hex_str = hex::encode(&bytes);
            unsafe { push(stack, Value::String(global_string(hex_str))) }
        }
        _ => panic!("random-bytes: expected Int on stack, got {:?}", value),
    }
}

/// Generate a UUID v4 (random)
///
/// Stack effect: ( -- String )
///
/// Returns a UUID in standard format: "550e8400-e29b-41d4-a716-446655440000"
///
/// # Safety
/// Stack pointer must be valid
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_uuid4(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "uuid4: stack is empty");

    let uuid = Uuid::new_v4();
    unsafe { push(stack, Value::String(global_string(uuid.to_string()))) }
}

/// Generate a cryptographically secure random integer in a range
///
/// Stack effect: ( min max -- Int )
///
/// Returns a uniform random integer in the range [min, max).
/// Uses rejection sampling to avoid modulo bias.
///
/// # Edge Cases
/// - If min >= max, returns min
/// - Uses the same CSPRNG as crypto.random-bytes
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_random_int(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "random-int: stack is empty");

    let (stack, max_val) = unsafe { pop(stack) };
    let (stack, min_val) = unsafe { pop(stack) };

    match (min_val, max_val) {
        (Value::Int(min), Value::Int(max)) => {
            let result = if min >= max {
                min // Edge case: return min if range is empty or invalid
            } else {
                random_int_range(min, max)
            };
            unsafe { push(stack, Value::Int(result)) }
        }
        (min, max) => panic!(
            "random-int: expected (Int, Int) on stack, got ({:?}, {:?})",
            min, max
        ),
    }
}

/// Generate a uniform random integer in [min, max) using rejection sampling
///
/// This avoids modulo bias by rejecting values that would cause uneven distribution.
fn random_int_range(min: i64, max: i64) -> i64 {
    // Use wrapping subtraction in unsigned space to handle full i64 range
    // without overflow (e.g., min=i64::MIN, max=i64::MAX would overflow in signed)
    let range = (max as u64).wrapping_sub(min as u64);
    if range == 0 {
        return min;
    }

    // Use rejection sampling to get unbiased result
    // For ranges that are powers of 2, no rejection needed
    // For other ranges, we reject values >= (u64::MAX - (u64::MAX % range))
    // to ensure uniform distribution
    let threshold = u64::MAX - (u64::MAX % range);

    loop {
        // Generate random u64 using fill_bytes (same CSPRNG as random_bytes)
        let mut bytes = [0u8; 8];
        rng().fill_bytes(&mut bytes);
        let val = u64::from_le_bytes(bytes);

        if val < threshold {
            // Add offset to min using unsigned arithmetic to avoid overflow
            // when min is negative and offset is large
            let result = (min as u64).wrapping_add(val % range);
            return result as i64;
        }
        // Rejection: try again (very rare, < 1 in 2^63 for most ranges)
    }
}

/// Encrypt plaintext using AES-256-GCM
///
/// Stack effect: ( String String -- String Bool )
///
/// Arguments:
/// - plaintext: The string to encrypt
/// - key: Hex-encoded 32-byte key (64 hex characters)
///
/// Returns:
/// - ciphertext: base64(nonce || ciphertext || tag)
/// - success: Bool indicating success
///
/// # Safety
/// Stack must have two String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_crypto_aes_gcm_encrypt(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "crypto.aes-gcm-encrypt: stack is null");

    let (stack, key_val) = unsafe { pop(stack) };
    let (stack, plaintext_val) = unsafe { pop(stack) };

    match (plaintext_val, key_val) {
        (Value::String(plaintext), Value::String(key_hex)) => {
            match aes_gcm_encrypt(plaintext.as_str(), key_hex.as_str()) {
                Some(ciphertext) => {
                    let stack = unsafe { push(stack, Value::String(global_string(ciphertext))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                None => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            }
        }
        _ => panic!("crypto.aes-gcm-encrypt: expected two Strings on stack"),
    }
}

/// Decrypt ciphertext using AES-256-GCM
///
/// Stack effect: ( String String -- String Bool )
///
/// Arguments:
/// - ciphertext: base64(nonce || ciphertext || tag)
/// - key: Hex-encoded 32-byte key (64 hex characters)
///
/// Returns:
/// - plaintext: The decrypted string
/// - success: Bool indicating success
///
/// # Safety
/// Stack must have two String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_crypto_aes_gcm_decrypt(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "crypto.aes-gcm-decrypt: stack is null");

    let (stack, key_val) = unsafe { pop(stack) };
    let (stack, ciphertext_val) = unsafe { pop(stack) };

    match (ciphertext_val, key_val) {
        (Value::String(ciphertext), Value::String(key_hex)) => {
            match aes_gcm_decrypt(ciphertext.as_str(), key_hex.as_str()) {
                Some(plaintext) => {
                    let stack = unsafe { push(stack, Value::String(global_string(plaintext))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                None => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            }
        }
        _ => panic!("crypto.aes-gcm-decrypt: expected two Strings on stack"),
    }
}

/// Derive a key from a password using PBKDF2-SHA256
///
/// Stack effect: ( String String Int -- String Bool )
///
/// Arguments:
/// - password: The password string
/// - salt: Salt string (should be unique per user/context)
/// - iterations: Number of iterations (recommend 100000+)
///
/// Returns:
/// - key: Hex-encoded 32-byte key (64 hex characters)
/// - success: Bool indicating success
///
/// # Safety
/// Stack must have String, String, Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_crypto_pbkdf2_sha256(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "crypto.pbkdf2-sha256: stack is null");

    let (stack, iterations_val) = unsafe { pop(stack) };
    let (stack, salt_val) = unsafe { pop(stack) };
    let (stack, password_val) = unsafe { pop(stack) };

    match (password_val, salt_val, iterations_val) {
        (Value::String(password), Value::String(salt), Value::Int(iterations)) => {
            // Require minimum iterations for security (100,000+ recommended for production)
            if iterations < MIN_PBKDF2_ITERATIONS {
                let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                return unsafe { push(stack, Value::Bool(false)) };
            }

            let key = derive_key_pbkdf2(password.as_str(), salt.as_str(), iterations as u32);
            let key_hex = hex::encode(key);
            let stack = unsafe { push(stack, Value::String(global_string(key_hex))) };
            unsafe { push(stack, Value::Bool(true)) }
        }
        _ => panic!("crypto.pbkdf2-sha256: expected String, String, Int on stack"),
    }
}

// Helper functions for AES-GCM

fn aes_gcm_encrypt(plaintext: &str, key_hex: &str) -> Option<String> {
    // Decode hex key
    let key_bytes = hex::decode(key_hex).ok()?;
    if key_bytes.len() != AES_KEY_SIZE {
        return None;
    }

    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).ok()?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; AES_NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).ok()?;

    // Combine: nonce || ciphertext (tag is appended by aes-gcm)
    let mut combined = Vec::with_capacity(AES_NONCE_SIZE + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Some(STANDARD.encode(&combined))
}

fn aes_gcm_decrypt(ciphertext_b64: &str, key_hex: &str) -> Option<String> {
    // Decode base64
    let combined = STANDARD.decode(ciphertext_b64).ok()?;
    if combined.len() < AES_NONCE_SIZE + AES_GCM_TAG_SIZE {
        // At minimum: nonce + tag
        return None;
    }

    // Decode hex key
    let key_bytes = hex::decode(key_hex).ok()?;
    if key_bytes.len() != AES_KEY_SIZE {
        return None;
    }

    // Split nonce and ciphertext
    let (nonce_bytes, ciphertext) = combined.split_at(AES_NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Create cipher and decrypt
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).ok()?;
    let plaintext_bytes = cipher.decrypt(nonce, ciphertext).ok()?;

    String::from_utf8(plaintext_bytes).ok()
}

fn derive_key_pbkdf2(password: &str, salt: &str, iterations: u32) -> [u8; AES_KEY_SIZE] {
    let mut key = [0u8; AES_KEY_SIZE];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), iterations, &mut key);
    key
}

// ============================================================================
// Ed25519 Digital Signatures
// ============================================================================

/// Generate an Ed25519 keypair
///
/// Stack effect: ( -- public-key private-key )
///
/// Returns:
/// - public-key: Hex-encoded 32-byte public key (64 hex characters)
/// - private-key: Hex-encoded 32-byte private key (64 hex characters)
///
/// # Safety
/// Stack must be valid
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_crypto_ed25519_keypair(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "crypto.ed25519-keypair: stack is null");

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());

    let stack = unsafe { push(stack, Value::String(global_string(public_hex))) };
    unsafe { push(stack, Value::String(global_string(private_hex))) }
}

/// Sign a message with an Ed25519 private key
///
/// Stack effect: ( message private-key -- signature success )
///
/// Parameters:
/// - message: The message to sign (any string)
/// - private-key: Hex-encoded 32-byte private key (64 hex characters)
///
/// Returns:
/// - signature: Hex-encoded 64-byte signature (128 hex characters)
/// - success: Bool indicating success
///
/// # Safety
/// Stack must have String, String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_crypto_ed25519_sign(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "crypto.ed25519-sign: stack is null");

    let (stack, key_val) = unsafe { pop(stack) };
    let (stack, msg_val) = unsafe { pop(stack) };

    match (msg_val, key_val) {
        (Value::String(message), Value::String(private_key_hex)) => {
            match ed25519_sign(message.as_str(), private_key_hex.as_str()) {
                Some(signature) => {
                    let stack = unsafe { push(stack, Value::String(global_string(signature))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                None => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            }
        }
        _ => panic!("crypto.ed25519-sign: expected String, String on stack"),
    }
}

/// Verify an Ed25519 signature
///
/// Stack effect: ( message signature public-key -- valid )
///
/// Parameters:
/// - message: The original message
/// - signature: Hex-encoded 64-byte signature (128 hex characters)
/// - public-key: Hex-encoded 32-byte public key (64 hex characters)
///
/// Returns:
/// - valid: Bool indicating whether the signature is valid
///
/// # Safety
/// Stack must have String, String, String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_crypto_ed25519_verify(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "crypto.ed25519-verify: stack is null");

    let (stack, pubkey_val) = unsafe { pop(stack) };
    let (stack, sig_val) = unsafe { pop(stack) };
    let (stack, msg_val) = unsafe { pop(stack) };

    match (msg_val, sig_val, pubkey_val) {
        (Value::String(message), Value::String(signature_hex), Value::String(public_key_hex)) => {
            let valid = ed25519_verify(
                message.as_str(),
                signature_hex.as_str(),
                public_key_hex.as_str(),
            );
            unsafe { push(stack, Value::Bool(valid)) }
        }
        _ => panic!("crypto.ed25519-verify: expected String, String, String on stack"),
    }
}

// Helper functions for Ed25519

fn ed25519_sign(message: &str, private_key_hex: &str) -> Option<String> {
    let key_bytes = hex::decode(private_key_hex).ok()?;
    if key_bytes.len() != 32 {
        return None;
    }

    let key_array: [u8; 32] = key_bytes.try_into().ok()?;
    let signing_key = SigningKey::from_bytes(&key_array);
    let signature = signing_key.sign(message.as_bytes());

    Some(hex::encode(signature.to_bytes()))
}

fn ed25519_verify(message: &str, signature_hex: &str, public_key_hex: &str) -> bool {
    let verify_inner = || -> Option<bool> {
        let sig_bytes = hex::decode(signature_hex).ok()?;
        if sig_bytes.len() != 64 {
            return Some(false);
        }

        let pubkey_bytes = hex::decode(public_key_hex).ok()?;
        if pubkey_bytes.len() != 32 {
            return Some(false);
        }

        let sig_array: [u8; 64] = sig_bytes.try_into().ok()?;
        let pubkey_array: [u8; 32] = pubkey_bytes.try_into().ok()?;

        let signature = Signature::from_bytes(&sig_array);
        let verifying_key = VerifyingKey::from_bytes(&pubkey_array).ok()?;

        Some(verifying_key.verify(message.as_bytes(), &signature).is_ok())
    };

    verify_inner().unwrap_or(false)
}

#[cfg(test)]
mod tests;
