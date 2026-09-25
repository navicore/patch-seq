//! Stub module for TLS client operations when the "http" feature is
//! disabled. Mirrors crypto_stub/http_stub: same FFI symbol, helpful
//! panic instead of capability code. Base builds contain no rustls at
//! all (see docs/design/RUNTIME_CAPABILITY_LINKING.md).

use seq_core::stack::Stack;

const FEATURE_MSG: &str = "http feature not enabled. Rebuild with: cargo build --features http";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_tls_client(_stack: Stack) -> Stack {
    panic!("net.tls.client requires {}", FEATURE_MSG);
}
