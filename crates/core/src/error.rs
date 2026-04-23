//! Runtime Error Handling
//!
//! Provides thread-local error state for FFI functions to report errors
//! without panicking across the FFI boundary.
//!
//! # Usage
//!
//! FFI functions can set an error instead of panicking:
//! ```ignore
//! if divisor == 0 {
//!     set_runtime_error("divide: division by zero");
//!     return stack; // Return unchanged stack
//! }
//! ```
//!
//! Callers can check for errors:
//! ```ignore
//! if patch_seq_has_error() {
//!     let error = patch_seq_take_error();
//!     // Handle error...
//! }
//! ```

use std::cell::RefCell;
use std::ffi::CString;
use std::ptr;

thread_local! {
    /// Thread-local storage for the last runtime error message
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };

    /// Cached C string for FFI access (avoids allocation on every get)
    static ERROR_CSTRING: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Set the last runtime error message
///
/// Note: This clears any cached CString to prevent stale pointer access.
pub fn set_runtime_error(msg: impl Into<String>) {
    // Clear cached CString first to prevent stale pointers
    ERROR_CSTRING.with(|cs| *cs.borrow_mut() = None);
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = Some(msg.into());
    });
}

/// Take (and clear) the last runtime error message
pub fn take_runtime_error() -> Option<String> {
    LAST_ERROR.with(|e| e.borrow_mut().take())
}

/// Check if there's a pending runtime error
pub fn has_runtime_error() -> bool {
    LAST_ERROR.with(|e| e.borrow().is_some())
}

/// Clear any pending runtime error
pub fn clear_runtime_error() {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
    ERROR_CSTRING.with(|e| *e.borrow_mut() = None);
}

// FFI-safe error access functions

/// Replace any interior null bytes with `'?'`, build a `CString`, cache it
/// in `ERROR_CSTRING`, and return a raw pointer into the cached string.
///
/// The returned pointer is valid until the next call to `set_runtime_error`,
/// `patch_seq_get_error`, `patch_seq_take_error`, or `patch_seq_clear_error`
/// replaces or clears the cached `CString`.
fn cache_error_cstring(msg: &str) -> *const i8 {
    let safe_msg: String = msg
        .chars()
        .map(|c| if c == '\0' { '?' } else { c })
        .collect();
    let cstring = CString::new(safe_msg).expect("null bytes already replaced");
    ERROR_CSTRING.with(|cs| {
        let ptr = cstring.as_ptr();
        *cs.borrow_mut() = Some(cstring);
        ptr
    })
}

/// Check if there's a pending runtime error (FFI-safe)
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_has_error() -> bool {
    has_runtime_error()
}

/// Get the last error message as a C string pointer (FFI-safe)
///
/// Returns null if no error is pending.
///
/// # WARNING: Pointer Lifetime
/// The returned pointer is only valid until the next call to `set_runtime_error`,
/// `get_error`, `take_error`, or `clear_error`. Callers must copy the string
/// immediately if they need to retain it.
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_get_error() -> *const i8 {
    LAST_ERROR.with(|e| match e.borrow().as_deref() {
        Some(msg) => cache_error_cstring(msg),
        None => ptr::null(),
    })
}

/// Take (and clear) the last error, returning it as a C string (FFI-safe)
///
/// Returns null if no error is pending.
///
/// # WARNING: Pointer Lifetime
/// The returned pointer is only valid until the next call to `set_runtime_error`,
/// `get_error`, `take_error`, or `clear_error`. Callers must copy the string
/// immediately if they need to retain it.
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_take_error() -> *const i8 {
    match take_runtime_error() {
        Some(msg) => cache_error_cstring(&msg),
        None => ptr::null(),
    }
}

/// Clear any pending error (FFI-safe)
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_clear_error() {
    clear_runtime_error();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_take_error() {
        clear_runtime_error();
        assert!(!has_runtime_error());

        set_runtime_error("test error");
        assert!(has_runtime_error());

        let error = take_runtime_error();
        assert_eq!(error, Some("test error".to_string()));
        assert!(!has_runtime_error());
    }

    #[test]
    fn test_clear_error() {
        set_runtime_error("another error");
        assert!(has_runtime_error());

        clear_runtime_error();
        assert!(!has_runtime_error());
        assert!(take_runtime_error().is_none());
    }
}
