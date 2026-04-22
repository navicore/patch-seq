//! AES-256-GCM authenticated encryption.

use crate::seqstring::global_string;
use crate::stack::{Stack, pop, push};
use crate::value::Value;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit as AesKeyInit, OsRng, rand_core::RngCore as AeadRngCore},
};
use base64::{Engine, engine::general_purpose::STANDARD};

use super::{AES_GCM_TAG_SIZE, AES_KEY_SIZE, AES_NONCE_SIZE};

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

pub(super) fn aes_gcm_encrypt(plaintext: &str, key_hex: &str) -> Option<String> {
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

pub(super) fn aes_gcm_decrypt(ciphertext_b64: &str, key_hex: &str) -> Option<String> {
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
