//! Cryptographic operations for Seq
//!
//! FFI entry points are grouped into per-primitive sub-modules and
//! re-exported here so the flat public surface is unchanged.
//!
//! # API
//!
//! - `sha256`, `hmac-sha256`, `constant-time-eq` — `hash`
//! - `random-bytes`, `uuid4`, `random-int` — `random`
//! - `aes-gcm-encrypt`, `aes-gcm-decrypt` — `aes`
//! - `pbkdf2-sha256` — `pbkdf`
//! - `ed25519-keypair`, `ed25519-sign`, `ed25519-verify` — `ed25519`

pub(super) const AES_NONCE_SIZE: usize = 12;
pub(super) const AES_KEY_SIZE: usize = 32;
pub(super) const AES_GCM_TAG_SIZE: usize = 16;
pub(super) const MIN_PBKDF2_ITERATIONS: i64 = 1_000;

mod aes;
mod ed25519;
mod hash;
mod pbkdf;
mod random;

#[cfg(test)]
mod tests;

pub use aes::*;
pub use ed25519::*;
pub use hash::*;
pub use pbkdf::*;
pub use random::*;
