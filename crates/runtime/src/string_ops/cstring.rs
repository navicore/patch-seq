//! FFI C-string glue: Seq String ↔ null-terminated C char buffer.
//! Used by the FFI wrapper codegen to pass strings across the C ABI.

use crate::seqstring::global_string;
use crate::stack::{Stack, push};
use crate::value::Value;

// ============================================================================
// FFI String Helpers
// ============================================================================

/// Convert a Seq String on the stack to a null-terminated C string.
///
/// The returned pointer must be freed by the caller using free().
/// This peeks the string from the stack (caller pops after use).
///
/// Stack effect: ( String -- ) returns ptr to C string
///
/// # Memory Safety
///
/// The returned C string is a **completely independent copy** allocated via
/// `malloc()`. It has no connection to Seq's memory management:
///
/// - The Seq String on the stack remains valid and unchanged
/// - The returned pointer is owned by the C world and must be freed with `free()`
/// - Even if the Seq String is garbage collected, the C string remains valid
/// - Multiple calls with the same Seq String produce independent C strings
///
/// This design ensures FFI calls cannot cause use-after-free or double-free
/// bugs between Seq and C code.
///
/// # Safety
/// Stack must have a String value on top. The unused second argument
/// exists for future extension (passing output buffer).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_to_cstring(stack: Stack, _out: *mut u8) -> *mut u8 {
    assert!(!stack.is_null(), "string_to_cstring: stack is empty");

    use crate::stack::peek;
    use crate::value::Value;

    // Peek the string value (don't pop - caller will pop after we return)
    let val = unsafe { peek(stack) };
    let s = match &val {
        Value::String(s) => s,
        other => panic!(
            "string_to_cstring: expected String on stack, got {:?}",
            other
        ),
    };

    let str_ptr = s.as_ptr();
    let len = s.len();

    // Guard against overflow: len + 1 for null terminator
    let alloc_size = len.checked_add(1).unwrap_or_else(|| {
        panic!(
            "string_to_cstring: string too large for C conversion (len={})",
            len
        )
    });

    // Allocate space for string + null terminator
    let ptr = unsafe { libc::malloc(alloc_size) as *mut u8 };
    if ptr.is_null() {
        panic!("string_to_cstring: malloc failed");
    }

    // Copy string data
    unsafe {
        std::ptr::copy_nonoverlapping(str_ptr, ptr, len);
        // Add null terminator
        *ptr.add(len) = 0;
    }

    ptr
}

/// Convert a null-terminated C string to a Seq String and push onto stack.
///
/// The C string is NOT freed by this function.
///
/// Stack effect: ( -- String )
///
/// # Safety
/// cstr must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_cstring_to_string(stack: Stack, cstr: *const u8) -> Stack {
    if cstr.is_null() {
        // NULL string - push empty string
        return unsafe { push(stack, Value::String(global_string(String::new()))) };
    }

    // Get string length
    let len = unsafe { libc::strlen(cstr as *const libc::c_char) };

    // Create Rust string from C string
    let slice = unsafe { std::slice::from_raw_parts(cstr, len) };
    let s = String::from_utf8_lossy(slice).into_owned();

    unsafe { push(stack, Value::String(global_string(s))) }
}
