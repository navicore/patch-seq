//! Compression operations for Seq
//!
//! These functions are exported with C ABI for LLVM codegen to call.
//! Uses flate2 for gzip and zstd for Zstandard compression.
//!
//! Compressed data is returned as base64-encoded strings for easy
//! storage and transmission in string-based contexts.
//!
//! # API
//!
//! ```seq
//! # Gzip compression (base64-encoded output)
//! "hello world" compress.gzip           # ( String -- String Bool )
//! compressed compress.gunzip            # ( String -- String Bool )
//!
//! # Gzip with compression level (1-9, higher = smaller but slower)
//! "hello world" 9 compress.gzip-level   # ( String Int -- String Bool )
//!
//! # Zstd compression (faster, better ratios)
//! "hello world" compress.zstd           # ( String -- String Bool )
//! compressed compress.unzstd            # ( String -- String Bool )
//!
//! # Zstd with compression level (1-22, default is 3)
//! "hello world" 19 compress.zstd-level  # ( String Int -- String Bool )
//! ```

use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use seq_core::seqstring::global_string;
use seq_core::stack::{Stack, pop, push};
use seq_core::value::Value;
use std::io::Read;

/// Compress data using gzip with default compression level (6)
///
/// Stack effect: ( String -- String Bool )
///
/// Returns base64-encoded compressed data and success flag.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_compress_gzip(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "compress.gzip: stack is null");

    let (stack, data_val) = unsafe { pop(stack) };

    match data_val {
        Value::String(data) => {
            match gzip_compress(data.as_str().as_bytes(), Compression::default()) {
                Some(compressed) => {
                    let encoded = STANDARD.encode(&compressed);
                    let stack = unsafe { push(stack, Value::String(global_string(encoded))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                None => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            }
        }
        _ => panic!("compress.gzip: expected String on stack"),
    }
}

/// Compress data using gzip with specified compression level
///
/// Stack effect: ( String Int -- String Bool )
///
/// Level should be 1-9 (1=fastest, 9=best compression).
/// Returns base64-encoded compressed data and success flag.
///
/// # Safety
/// Stack must have Int and String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_compress_gzip_level(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "compress.gzip-level: stack is null");

    let (stack, level_val) = unsafe { pop(stack) };
    let (stack, data_val) = unsafe { pop(stack) };

    match (data_val, level_val) {
        (Value::String(data), Value::Int(level)) => {
            let level = level.clamp(1, 9) as u32;
            match gzip_compress(data.as_str().as_bytes(), Compression::new(level)) {
                Some(compressed) => {
                    let encoded = STANDARD.encode(&compressed);
                    let stack = unsafe { push(stack, Value::String(global_string(encoded))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                None => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            }
        }
        _ => panic!("compress.gzip-level: expected String and Int on stack"),
    }
}

/// Decompress gzip data
///
/// Stack effect: ( String -- String Bool )
///
/// Input should be base64-encoded gzip data.
/// Returns decompressed string and success flag.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_compress_gunzip(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "compress.gunzip: stack is null");

    let (stack, data_val) = unsafe { pop(stack) };

    match data_val {
        Value::String(data) => {
            // Decode base64
            let decoded = match STANDARD.decode(data.as_str()) {
                Ok(d) => d,
                Err(_) => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    return unsafe { push(stack, Value::Bool(false)) };
                }
            };

            // Decompress
            match gzip_decompress(&decoded) {
                Some(decompressed) => {
                    let stack = unsafe { push(stack, Value::String(global_string(decompressed))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                None => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            }
        }
        _ => panic!("compress.gunzip: expected String on stack"),
    }
}

/// Compress data using zstd with default compression level (3)
///
/// Stack effect: ( String -- String Bool )
///
/// Returns base64-encoded compressed data and success flag.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_compress_zstd(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "compress.zstd: stack is null");

    let (stack, data_val) = unsafe { pop(stack) };

    match data_val {
        Value::String(data) => match zstd::encode_all(data.as_str().as_bytes(), 3) {
            Ok(compressed) => {
                let encoded = STANDARD.encode(&compressed);
                let stack = unsafe { push(stack, Value::String(global_string(encoded))) };
                unsafe { push(stack, Value::Bool(true)) }
            }
            Err(_) => {
                let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                unsafe { push(stack, Value::Bool(false)) }
            }
        },
        _ => panic!("compress.zstd: expected String on stack"),
    }
}

/// Compress data using zstd with specified compression level
///
/// Stack effect: ( String Int -- String Bool )
///
/// Level should be 1-22 (higher = better compression but slower).
/// Returns base64-encoded compressed data and success flag.
///
/// # Safety
/// Stack must have Int and String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_compress_zstd_level(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "compress.zstd-level: stack is null");

    let (stack, level_val) = unsafe { pop(stack) };
    let (stack, data_val) = unsafe { pop(stack) };

    match (data_val, level_val) {
        (Value::String(data), Value::Int(level)) => {
            let level = level.clamp(1, 22) as i32;
            match zstd::encode_all(data.as_str().as_bytes(), level) {
                Ok(compressed) => {
                    let encoded = STANDARD.encode(&compressed);
                    let stack = unsafe { push(stack, Value::String(global_string(encoded))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                Err(_) => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            }
        }
        _ => panic!("compress.zstd-level: expected String and Int on stack"),
    }
}

/// Decompress zstd data
///
/// Stack effect: ( String -- String Bool )
///
/// Input should be base64-encoded zstd data.
/// Returns decompressed string and success flag.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_compress_unzstd(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "compress.unzstd: stack is null");

    let (stack, data_val) = unsafe { pop(stack) };

    match data_val {
        Value::String(data) => {
            // Decode base64
            let decoded = match STANDARD.decode(data.as_str()) {
                Ok(d) => d,
                Err(_) => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    return unsafe { push(stack, Value::Bool(false)) };
                }
            };

            // Decompress
            match zstd::decode_all(decoded.as_slice()) {
                Ok(decompressed) => match String::from_utf8(decompressed) {
                    Ok(s) => {
                        let stack = unsafe { push(stack, Value::String(global_string(s))) };
                        unsafe { push(stack, Value::Bool(true)) }
                    }
                    Err(_) => {
                        let stack =
                            unsafe { push(stack, Value::String(global_string(String::new()))) };
                        unsafe { push(stack, Value::Bool(false)) }
                    }
                },
                Err(_) => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            }
        }
        _ => panic!("compress.unzstd: expected String on stack"),
    }
}

// Helper functions

fn gzip_compress(data: &[u8], level: Compression) -> Option<Vec<u8>> {
    let mut encoder = GzEncoder::new(data, level);
    let mut compressed = Vec::new();
    match encoder.read_to_end(&mut compressed) {
        Ok(_) => Some(compressed),
        Err(_) => None,
    }
}

fn gzip_decompress(data: &[u8]) -> Option<String> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = String::new();
    match decoder.read_to_string(&mut decompressed) {
        Ok(_) => Some(decompressed),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests;
