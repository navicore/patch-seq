use super::*;
use crate::stack::pop;

#[test]
fn test_sha256() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_string())));
        let stack = patch_seq_sha256(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => {
                // SHA-256 of "hello"
                assert_eq!(
                    s.as_str(),
                    "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
                );
            }
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_sha256_empty() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string(String::new())));
        let stack = patch_seq_sha256(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => {
                // SHA-256 of empty string
                assert_eq!(
                    s.as_str(),
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                );
            }
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_hmac_sha256() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("message".to_string())));
        let stack = push(stack, Value::String(global_string("secret".to_string())));
        let stack = patch_seq_hmac_sha256(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => {
                // HMAC-SHA256("message", "secret")
                assert_eq!(
                    s.as_str(),
                    "8b5f48702995c1598c573db1e21866a9b825d4a794d169d7060a03605796360b"
                );
            }
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_constant_time_eq_equal() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_string())));
        let stack = push(stack, Value::String(global_string("hello".to_string())));
        let stack = patch_seq_constant_time_eq(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Bool(b) => assert!(b),
            _ => panic!("Expected Bool"),
        }
    }
}

#[test]
fn test_constant_time_eq_different() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_string())));
        let stack = push(stack, Value::String(global_string("world".to_string())));
        let stack = patch_seq_constant_time_eq(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Bool(b) => assert!(!b),
            _ => panic!("Expected Bool"),
        }
    }
}

#[test]
fn test_constant_time_eq_different_lengths() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_string())));
        let stack = push(stack, Value::String(global_string("hi".to_string())));
        let stack = patch_seq_constant_time_eq(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Bool(b) => assert!(!b),
            _ => panic!("Expected Bool"),
        }
    }
}

#[test]
fn test_random_bytes() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(16));
        let stack = patch_seq_random_bytes(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => {
                // 16 bytes = 32 hex chars
                assert_eq!(s.as_str().len(), 32);
                // Should be valid hex
                assert!(hex::decode(s.as_str()).is_ok());
            }
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_random_bytes_zero() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(0));
        let stack = patch_seq_random_bytes(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => {
                assert_eq!(s.as_str(), "");
            }
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_uuid4() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = patch_seq_uuid4(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => {
                // UUID format: 8-4-4-4-12
                assert_eq!(s.as_str().len(), 36);
                assert_eq!(s.as_str().chars().filter(|c| *c == '-').count(), 4);
                // Version 4 indicator at position 14
                assert_eq!(s.as_str().chars().nth(14), Some('4'));
            }
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_uuid4_unique() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = patch_seq_uuid4(stack);
        let (stack, value1) = pop(stack);
        let stack = patch_seq_uuid4(stack);
        let (_, value2) = pop(stack);

        match (value1, value2) {
            (Value::String(s1), Value::String(s2)) => {
                assert_ne!(s1.as_str(), s2.as_str());
            }
            _ => panic!("Expected Strings"),
        }
    }
}

#[test]
fn test_random_bytes_max_limit() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(1024)); // Max allowed
        let stack = patch_seq_random_bytes(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => {
                // 1024 bytes = 2048 hex chars
                assert_eq!(s.as_str().len(), 2048);
            }
            _ => panic!("Expected String"),
        }
    }
}
// Note: Exceeding the 1024 byte limit causes a panic, which aborts in FFI context.
// This is intentional - the limit prevents memory exhaustion attacks.

// AES-GCM Tests

#[test]
fn test_aes_gcm_roundtrip() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();

        // Create a test key (32 bytes = 64 hex chars)
        let key_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

        let stack = push(
            stack,
            Value::String(global_string("hello world".to_string())),
        );
        let stack = push(stack, Value::String(global_string(key_hex.to_string())));

        // Encrypt
        let stack = patch_seq_crypto_aes_gcm_encrypt(stack);

        // Check encrypt success
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));

        // Add key for decrypt
        let stack = push(stack, Value::String(global_string(key_hex.to_string())));

        // Decrypt
        let stack = patch_seq_crypto_aes_gcm_decrypt(stack);

        // Check decrypt success
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));

        // Check plaintext
        let (_, result) = pop(stack);
        if let Value::String(s) = result {
            assert_eq!(s.as_str(), "hello world");
        } else {
            panic!("expected string");
        }
    }
}

#[test]
fn test_aes_gcm_wrong_key() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();

        let key1 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let key2 = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

        let stack = push(
            stack,
            Value::String(global_string("secret message".to_string())),
        );
        let stack = push(stack, Value::String(global_string(key1.to_string())));

        // Encrypt with key1
        let stack = patch_seq_crypto_aes_gcm_encrypt(stack);
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));

        // Try to decrypt with key2
        let stack = push(stack, Value::String(global_string(key2.to_string())));
        let stack = patch_seq_crypto_aes_gcm_decrypt(stack);

        // Should fail
        let (_, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
    }
}

#[test]
fn test_aes_gcm_invalid_key_length() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();

        // Key too short
        let short_key = "0123456789abcdef";

        let stack = push(stack, Value::String(global_string("test data".to_string())));
        let stack = push(stack, Value::String(global_string(short_key.to_string())));

        let stack = patch_seq_crypto_aes_gcm_encrypt(stack);
        let (_, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
    }
}

#[test]
fn test_aes_gcm_invalid_ciphertext() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();

        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

        // Invalid base64
        let stack = push(
            stack,
            Value::String(global_string("not-valid-base64!!!".to_string())),
        );
        let stack = push(stack, Value::String(global_string(key.to_string())));

        let stack = patch_seq_crypto_aes_gcm_decrypt(stack);
        let (_, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
    }
}

#[test]
fn test_aes_gcm_empty_plaintext() {
    let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    let ciphertext = aes_gcm_encrypt("", key).unwrap();
    let decrypted = aes_gcm_decrypt(&ciphertext, key).unwrap();
    assert_eq!(decrypted, "");
}

#[test]
fn test_aes_gcm_special_characters() {
    let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let plaintext = "Hello\nWorld\tTab\"Quote\\Backslash";

    let ciphertext = aes_gcm_encrypt(plaintext, key).unwrap();
    let decrypted = aes_gcm_decrypt(&ciphertext, key).unwrap();
    assert_eq!(decrypted, plaintext);
}

// PBKDF2 Tests

#[test]
fn test_pbkdf2_sha256() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();

        let stack = push(
            stack,
            Value::String(global_string("my-password".to_string())),
        );
        let stack = push(
            stack,
            Value::String(global_string("random-salt".to_string())),
        );
        let stack = push(stack, Value::Int(10000));

        let stack = patch_seq_crypto_pbkdf2_sha256(stack);

        // Check success
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));

        // Check key is 64 hex chars (32 bytes)
        let (_, result) = pop(stack);
        if let Value::String(s) = result {
            assert_eq!(s.as_str().len(), 64);
            // Verify it's valid hex
            assert!(hex::decode(s.as_str()).is_ok());
        } else {
            panic!("expected string");
        }
    }
}

#[test]
fn test_pbkdf2_deterministic() {
    // Same inputs should produce same key
    let key1 = derive_key_pbkdf2("password", "salt", 10000);
    let key2 = derive_key_pbkdf2("password", "salt", 10000);
    assert_eq!(key1, key2);

    // Different inputs should produce different keys
    let key3 = derive_key_pbkdf2("password", "different-salt", 10000);
    assert_ne!(key1, key3);
}

#[test]
fn test_pbkdf2_invalid_iterations() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();

        let stack = push(stack, Value::String(global_string("password".to_string())));
        let stack = push(stack, Value::String(global_string("salt".to_string())));
        let stack = push(stack, Value::Int(0)); // Invalid: below minimum (1000)

        let stack = patch_seq_crypto_pbkdf2_sha256(stack);

        let (_, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
    }
}

#[test]
fn test_encrypt_decrypt_with_derived_key() {
    // Full workflow: derive key from password, then encrypt/decrypt
    let password = "user-secret-password";
    let salt = "unique-user-salt";
    let iterations = 10000u32;

    // Derive key
    let key = derive_key_pbkdf2(password, salt, iterations);
    let key_hex = hex::encode(key);

    // Encrypt
    let plaintext = "sensitive data to protect";
    let ciphertext = aes_gcm_encrypt(plaintext, &key_hex).unwrap();

    // Decrypt
    let decrypted = aes_gcm_decrypt(&ciphertext, &key_hex).unwrap();
    assert_eq!(decrypted, plaintext);
}

// Ed25519 tests

#[test]
fn test_ed25519_sign_verify() {
    let message = "Hello, World!";

    // Generate keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());

    // Sign
    let signature = ed25519_sign(message, &private_hex).unwrap();
    assert_eq!(signature.len(), 128); // 64 bytes = 128 hex chars

    // Verify
    assert!(ed25519_verify(message, &signature, &public_hex));
}

#[test]
fn test_ed25519_wrong_message() {
    let message = "Original message";
    let wrong_message = "Wrong message";

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());

    let signature = ed25519_sign(message, &private_hex).unwrap();

    // Verify with wrong message should fail
    assert!(!ed25519_verify(wrong_message, &signature, &public_hex));
}

#[test]
fn test_ed25519_wrong_key() {
    let message = "Test message";

    let signing_key1 = SigningKey::generate(&mut OsRng);
    let signing_key2 = SigningKey::generate(&mut OsRng);

    let private_hex = hex::encode(signing_key1.to_bytes());
    let wrong_public_hex = hex::encode(signing_key2.verifying_key().to_bytes());

    let signature = ed25519_sign(message, &private_hex).unwrap();

    // Verify with wrong public key should fail
    assert!(!ed25519_verify(message, &signature, &wrong_public_hex));
}

#[test]
fn test_ed25519_invalid_key_length() {
    let message = "Test message";
    let invalid_key = "tooshort";

    // Sign with invalid key should fail
    assert!(ed25519_sign(message, invalid_key).is_none());
}

#[test]
fn test_ed25519_invalid_signature() {
    let message = "Test message";

    let signing_key = SigningKey::generate(&mut OsRng);
    let public_hex = hex::encode(signing_key.verifying_key().to_bytes());

    let invalid_signature = "0".repeat(128); // Valid length but wrong signature

    // Verify with invalid signature should fail
    assert!(!ed25519_verify(message, &invalid_signature, &public_hex));
}

#[test]
fn test_ed25519_empty_message() {
    let message = "";

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());

    // Sign empty message
    let signature = ed25519_sign(message, &private_hex).unwrap();

    // Verify should succeed
    assert!(ed25519_verify(message, &signature, &public_hex));
}

#[test]
fn test_ed25519_keypair_ffi() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();

        let stack = patch_seq_crypto_ed25519_keypair(stack);

        let (stack, private_key) = pop(stack);
        let (_, public_key) = pop(stack);

        // Both should be 64-char hex strings (32 bytes)
        if let Value::String(pk) = public_key {
            assert_eq!(pk.as_str().len(), 64);
        } else {
            panic!("Expected String for public key");
        }

        if let Value::String(sk) = private_key {
            assert_eq!(sk.as_str().len(), 64);
        } else {
            panic!("Expected String for private key");
        }
    }
}

#[test]
fn test_ed25519_sign_ffi() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();

        // Generate a valid key first
        let signing_key = SigningKey::generate(&mut OsRng);
        let private_hex = hex::encode(signing_key.to_bytes());

        let stack = push(
            stack,
            Value::String(global_string("Test message".to_string())),
        );
        let stack = push(stack, Value::String(global_string(private_hex)));

        let stack = patch_seq_crypto_ed25519_sign(stack);

        let (stack, success) = pop(stack);
        let (_, signature) = pop(stack);

        assert_eq!(success, Value::Bool(true));
        if let Value::String(sig) = signature {
            assert_eq!(sig.as_str().len(), 128); // 64 bytes = 128 hex chars
        } else {
            panic!("Expected String for signature");
        }
    }
}

#[test]
fn test_ed25519_verify_ffi() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();

        // Generate keypair and sign
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let private_hex = hex::encode(signing_key.to_bytes());
        let public_hex = hex::encode(verifying_key.to_bytes());

        let message = "Verify this message";
        let signature = ed25519_sign(message, &private_hex).unwrap();

        let stack = push(stack, Value::String(global_string(message.to_string())));
        let stack = push(stack, Value::String(global_string(signature)));
        let stack = push(stack, Value::String(global_string(public_hex)));

        let stack = patch_seq_crypto_ed25519_verify(stack);

        let (_, valid) = pop(stack);
        assert_eq!(valid, Value::Bool(true));
    }
}

#[test]
fn test_random_int_basic() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(1));
        let stack = push(stack, Value::Int(100));
        let stack = patch_seq_random_int(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Int(n) => {
                assert!((1..100).contains(&n), "Expected 1 <= {} < 100", n);
            }
            _ => panic!("Expected Int"),
        }
    }
}

#[test]
fn test_random_int_same_min_max() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(5));
        let stack = push(stack, Value::Int(5));
        let stack = patch_seq_random_int(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Int(n) => assert_eq!(n, 5),
            _ => panic!("Expected Int"),
        }
    }
}

#[test]
fn test_random_int_inverted_range() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(10));
        let stack = push(stack, Value::Int(5));
        let stack = patch_seq_random_int(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Int(n) => assert_eq!(n, 10), // Returns min when min >= max
            _ => panic!("Expected Int"),
        }
    }
}

#[test]
fn test_random_int_small_range() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(0));
        let stack = push(stack, Value::Int(2));
        let stack = patch_seq_random_int(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Int(n) => assert!((0..2).contains(&n), "Expected 0 <= {} < 2", n),
            _ => panic!("Expected Int"),
        }
    }
}

#[test]
fn test_random_int_negative_range() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(-10));
        let stack = push(stack, Value::Int(10));
        let stack = patch_seq_random_int(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Int(n) => assert!((-10..10).contains(&n), "Expected -10 <= {} < 10", n),
            _ => panic!("Expected Int"),
        }
    }
}

#[test]
fn test_random_int_large_range() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(0));
        let stack = push(stack, Value::Int(i64::MAX));
        let stack = patch_seq_random_int(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Int(n) => assert!(n >= 0, "Expected {} >= 0", n),
            _ => panic!("Expected Int"),
        }
    }
}

#[test]
fn test_random_int_extreme_range() {
    // Test the overflow fix: min=i64::MIN, max=i64::MAX
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(i64::MIN));
        let stack = push(stack, Value::Int(i64::MAX));
        let stack = patch_seq_random_int(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Int(_) => {} // Any valid i64 is acceptable
            _ => panic!("Expected Int"),
        }
    }
}

#[test]
fn test_random_int_uniformity() {
    // Basic uniformity test: generate many samples and check distribution
    // For range [0, 10), each bucket should get roughly 10% of samples
    let mut buckets = [0u32; 10];
    let samples = 10000;

    unsafe {
        for _ in 0..samples {
            let stack = crate::stack::alloc_test_stack();
            let stack = push(stack, Value::Int(0));
            let stack = push(stack, Value::Int(10));
            let stack = patch_seq_random_int(stack);
            let (_, value) = pop(stack);

            if let Value::Int(n) = value {
                buckets[n as usize] += 1;
            }
        }
    }

    // Each bucket should have roughly 1000 samples (10%)
    // Allow 30% deviation (700-1300) to avoid flaky tests
    let expected = samples as u32 / 10;
    let tolerance = expected * 30 / 100;

    for (i, &count) in buckets.iter().enumerate() {
        assert!(
            count >= expected - tolerance && count <= expected + tolerance,
            "Bucket {} has {} samples, expected {} ± {} (uniformity test)",
            i,
            count,
            expected,
            tolerance
        );
    }
}
