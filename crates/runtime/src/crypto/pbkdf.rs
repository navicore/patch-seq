//! Password-based key derivation: PBKDF2-HMAC-SHA256.

use crate::seqstring::global_string;
use crate::stack::{Stack, pop, push};
use crate::value::Value;

use sha2::Sha256;

use super::{AES_KEY_SIZE, MIN_PBKDF2_ITERATIONS};

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

            // Password and salt are byte-clean — random bytes for
            // salts are common, and binary password material (e.g.,
            // pre-hashed input) survives unchanged.
            let key = derive_key_pbkdf2(password.as_bytes(), salt.as_bytes(), iterations as u32);
            let key_hex = hex::encode(key);
            let stack = unsafe { push(stack, Value::String(global_string(key_hex))) };
            unsafe { push(stack, Value::Bool(true)) }
        }
        _ => panic!("crypto.pbkdf2-sha256: expected String, String, Int on stack"),
    }
}

pub(super) fn derive_key_pbkdf2(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
) -> [u8; AES_KEY_SIZE] {
    let mut key = [0u8; AES_KEY_SIZE];
    pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, iterations, &mut key);
    key
}
