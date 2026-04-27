//! SeqString - Arena or Globally Allocated Byte String
//!
//! Strings in Seq are sequences of arbitrary bytes — there is **no
//! UTF-8 invariant** on this type. Byte-clean operations (concat,
//! length-in-bytes, channel send, network I/O, file I/O of binary
//! content, crypto inputs) work on any input. Text-level operations
//! (codepoint length, case folding, regex Unicode classes, JSON
//! escaping) call [`SeqString::as_str`] which validates UTF-8 at the
//! boundary and returns `Option<&str>`; on invalid bytes those ops
//! fail loudly with the standard `(value Bool)` failure tuple.
//!
//! The two allocation sources stay:
//! 1. Thread-local arena (fast, bulk-freed on strand exit)
//! 2. Global allocator (persists across arena resets, used for
//!    cross-strand transfer)
//!
//! See `docs/design/STRING_BYTE_CLEANLINESS.md` for the full design.

use crate::arena;
use std::fmt;

/// Byte string that tracks its allocation source.
///
/// # Safety Invariants
/// - If `global=true`: `ptr` points to a global-allocated byte buffer
///   whose memory matches `len`/`capacity`; the buffer is freed on
///   `Drop`.
/// - If `global=false`: `ptr` points into the thread-local arena; the
///   arena owns the memory and frees it in bulk on strand exit.
/// - The byte content is *not* required to be valid UTF-8.
/// - For global strings: `capacity` must match the original `Vec<u8>`'s
///   capacity so deallocation is correctly sized.
pub struct SeqString {
    ptr: *const u8,
    len: usize,
    capacity: usize, // Only meaningful for global strings
    global: bool,
}

// Implement PartialEq manually to compare content (bytes), not pointers.
impl PartialEq for SeqString {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for SeqString {}

// Safety: SeqString is Send because:
// - Global strings are truly independent (owned heap allocation)
// - Arena strings are cloned to global on channel send (see Clone impl)
// - We never send arena pointers across threads unsafely
unsafe impl Send for SeqString {}

// Safety: SeqString is Sync because:
// - The string content is immutable after construction
// - ptr/len are only read, never modified after construction
// - Global strings (Arc<String>) are already Sync
// - Arena strings point to memory that won't be deallocated while in use
unsafe impl Sync for SeqString {}

impl SeqString {
    /// Borrow the underlying bytes. Always succeeds; the type carries
    /// no UTF-8 invariant. Byte-clean operations (concat, byte
    /// length, equality, search, network I/O, crypto, etc.) should
    /// use this.
    pub fn as_bytes(&self) -> &[u8] {
        // Safety: `ptr` and `len` describe a valid byte buffer per the
        // `SeqString` invariants; the lifetime of the returned slice is
        // tied to `&self` so the buffer cannot be freed while the
        // slice is live.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    /// View as `&str` if the bytes happen to be valid UTF-8.
    ///
    /// Text-level operations (codepoint counting, case folding,
    /// `string.json-escape`, `regex.*` with Unicode classes,
    /// formatting for display) call this and treat `None` as a
    /// fallible-text failure — the conventional `(value Bool)`
    /// failure tuple, returning -1 / empty string / `false` per the
    /// surrounding op's contract.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.as_bytes()).ok()
    }

    /// View as `&str`, replacing any invalid UTF-8 with U+FFFD. Use
    /// only for human-facing display where lossiness is acceptable
    /// (Debug, panic messages, REPL output). Operations that round-
    /// trip user data must use [`as_bytes`] or [`as_str`] instead.
    pub fn as_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }

    /// View as `&str`, returning `""` if the bytes aren't valid UTF-8.
    ///
    /// The convenience for text-required ops (`string.length`,
    /// `string.find`, file paths, integer parsing, …) that expect a
    /// `&str` and have an existing degenerate-result-or-failure-tuple
    /// path for empty input. A non-UTF-8 input lands in that same
    /// failure path, with no extra branching at every call site.
    pub fn as_str_or_empty(&self) -> &str {
        self.as_str().unwrap_or("")
    }

    /// Check if this string is globally allocated
    pub fn is_global(&self) -> bool {
        self.global
    }

    /// Get length in bytes
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Check if this is an interned/static string (Issue #166)
    ///
    /// Interned strings have capacity=0 and point to static data.
    /// They are never freed and can be compared by pointer for O(1) equality.
    pub fn is_interned(&self) -> bool {
        self.capacity == 0 && self.global
    }

    /// Get raw pointer to string data
    ///
    /// Used for O(1) pointer comparison of interned symbols.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Reconstruct SeqString from raw parts
    ///
    /// # Safety
    /// The parts must be a valid allocation matching the ptr/len/capacity/global
    /// invariants documented on `SeqString`.
    pub unsafe fn from_raw_parts(
        ptr: *const u8,
        len: usize,
        capacity: usize,
        global: bool,
    ) -> Self {
        SeqString {
            ptr,
            len,
            capacity,
            global,
        }
    }
}

impl Clone for SeqString {
    /// Clone always allocates from the global allocator for Send safety.
    ///
    /// When a `SeqString` is sent through a channel, the receiving
    /// strand gets an independent global-allocated copy that doesn't
    /// depend on the sender's arena. Byte-clean: copies the underlying
    /// `&[u8]`, no UTF-8 validation.
    fn clone(&self) -> Self {
        global_bytes(self.as_bytes().to_vec())
    }
}

impl Drop for SeqString {
    fn drop(&mut self) {
        // Drop only if BOTH conditions are true:
        // - global=true: Arena strings have global=false and are bulk-freed on strand exit.
        // - capacity > 0: Interned symbols (Issue #166) have capacity=0 and point to
        //   static data that must NOT be deallocated.
        if self.global && self.capacity > 0 {
            // Reconstruct the owning `Vec<u8>` and drop it. Using
            // `Vec<u8>::from_raw_parts` (rather than `String::from_raw_parts`)
            // imposes no UTF-8 requirement on the buffer contents; deallocation
            // size is identical because `String` is just `Vec<u8>` plus a
            // UTF-8 invariant.
            //
            // Safety: We created this buffer in `global_bytes()` (via
            // `Vec::into_raw_parts`-equivalent) and stored the original
            // `ptr`, `len`, and `capacity`, so reconstruction is exact.
            unsafe {
                let _v = Vec::<u8>::from_raw_parts(self.ptr as *mut u8, self.len, self.capacity);
                // _v is dropped here, freeing the memory with correct size.
            }
        }
        // Arena strings don't need explicit drop — the arena's reset frees them.
        // Static/interned strings (capacity=0) point to static data — no drop needed.
    }
}

impl fmt::Debug for SeqString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Lossy display is fine here — Debug is for human consumption.
        write!(
            f,
            "SeqString({:?}, global={})",
            self.as_str_lossy(),
            self.global
        )
    }
}

impl fmt::Display for SeqString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display is human-facing; lossy-replace invalid UTF-8 with U+FFFD.
        // Round-trip data should use `as_bytes()` directly.
        write!(f, "{}", self.as_str_lossy())
    }
}

/// Create arena-allocated bytes (fast path for temporaries).
///
/// Accepts arbitrary bytes; no UTF-8 validation.
///
/// # Performance
/// ~5ns vs ~100ns for global allocator (20× faster).
///
/// # Lifetime
/// Valid until `arena_reset()` is called (typically when the strand exits).
pub fn arena_bytes(bytes: &[u8]) -> SeqString {
    arena::with_arena(|arena| {
        let arena_buf = arena.alloc_slice_copy(bytes);
        SeqString {
            ptr: arena_buf.as_ptr(),
            len: arena_buf.len(),
            capacity: 0, // Not used for arena strings
            global: false,
        }
    })
}

/// Create arena-allocated string from a UTF-8 `&str`. Convenience
/// wrapper over [`arena_bytes`] for callers that already have a Rust
/// `&str` in hand.
pub fn arena_string(s: &str) -> SeqString {
    arena_bytes(s.as_bytes())
}

/// Create globally-allocated bytes (persists across arena resets).
///
/// Accepts arbitrary bytes; no UTF-8 validation. Used when a
/// `SeqString` needs to outlive the current strand or cross a channel
/// boundary.
pub fn global_bytes(bytes: Vec<u8>) -> SeqString {
    let len = bytes.len();
    let capacity = bytes.capacity();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes); // Transfer ownership; Drop reconstructs and frees.

    SeqString {
        ptr,
        len,
        capacity,
        global: true,
    }
}

/// Create globally-allocated string from a UTF-8 `String`. Convenience
/// wrapper over [`global_bytes`] for callers that already have a Rust
/// `String` in hand.
pub fn global_string(s: String) -> SeqString {
    global_bytes(s.into_bytes())
}

/// Convert &str to SeqString using arena allocation
impl From<&str> for SeqString {
    fn from(s: &str) -> Self {
        arena_string(s)
    }
}

/// Convert String to SeqString using global allocation
impl From<String> for SeqString {
    fn from(s: String) -> Self {
        global_string(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_string() {
        let s = arena_string("Hello, arena!");
        assert_eq!(s.as_str(), Some("Hello, arena!"));
        assert_eq!(s.len(), 13);
        assert!(!s.is_global());
    }

    #[test]
    fn test_global_string() {
        let s = global_string("Hello, global!".to_string());
        assert_eq!(s.as_str(), Some("Hello, global!"));
        assert_eq!(s.len(), 14);
        assert!(s.is_global());
    }

    #[test]
    fn test_clone_creates_global() {
        // Clone an arena string
        let s1 = arena_string("test");
        let s2 = s1.clone();

        assert_eq!(s1.as_bytes(), s2.as_bytes());
        assert!(!s1.is_global());
        assert!(s2.is_global()); // Clone is always global!
    }

    #[test]
    fn test_clone_global() {
        let s1 = global_string("test".to_string());
        let s2 = s1.clone();

        assert_eq!(s1.as_bytes(), s2.as_bytes());
        assert!(s1.is_global());
        assert!(s2.is_global());
    }

    #[test]
    fn test_drop_global() {
        // Create and drop a global string
        {
            let s = global_string("Will be dropped".to_string());
            assert_eq!(s.as_str(), Some("Will be dropped"));
        }
        // If we get here without crashing, drop worked
    }

    #[test]
    fn test_drop_arena() {
        // Create and drop an arena string
        {
            let s = arena_string("Will be dropped (no-op)");
            assert_eq!(s.as_str(), Some("Will be dropped (no-op)"));
        }
        // Arena strings don't need explicit drop
    }

    #[test]
    fn test_equality() {
        let s1 = arena_string("test");
        let s2 = arena_string("test");
        let s3 = global_string("test".to_string());
        let s4 = arena_string("different");

        assert_eq!(s1, s2); // Same content, both arena
        assert_eq!(s1, s3); // Same content, different allocation
        assert_ne!(s1, s4); // Different content
    }

    #[test]
    fn test_from_str() {
        let s: SeqString = "test".into();
        assert_eq!(s.as_str(), Some("test"));
        assert!(!s.is_global()); // from &str uses arena
    }

    #[test]
    fn test_from_string() {
        let s: SeqString = "test".to_string().into();
        assert_eq!(s.as_str(), Some("test"));
        assert!(s.is_global()); // from String uses global
    }

    #[test]
    fn test_debug_format() {
        let s = arena_string("debug");
        let debug_str = format!("{:?}", s);
        assert!(debug_str.contains("debug"));
        assert!(debug_str.contains("global=false"));
    }

    #[test]
    fn test_display_format() {
        let s = global_string("display".to_string());
        let display_str = format!("{}", s);
        assert_eq!(display_str, "display");
    }

    #[test]
    fn test_empty_string() {
        let s = arena_string("");
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
        assert_eq!(s.as_str(), Some(""));
    }

    #[test]
    fn test_unicode() {
        let s = arena_string("Hello, 世界! 🦀");
        assert_eq!(s.as_str(), Some("Hello, 世界! 🦀"));
        assert!(s.len() > 10); // UTF-8 bytes, not chars
    }

    #[test]
    fn test_global_string_preserves_capacity() {
        // PR #11 Critical fix: Verify capacity is preserved for correct deallocation
        let mut s = String::with_capacity(100);
        s.push_str("hi");

        assert_eq!(s.len(), 2);
        assert_eq!(s.capacity(), 100);

        let cem = global_string(s);

        // Verify the SeqString captured the original capacity
        assert_eq!(cem.len(), 2);
        assert_eq!(cem.capacity, 100); // Critical: Must be 100, not 2!
        assert_eq!(cem.as_str(), Some("hi"));
        assert!(cem.is_global());

        // Drop cem - if capacity was wrong, this would cause heap corruption
        drop(cem);

        // If we get here without crash/UB, the fix worked
    }

    #[test]
    fn test_arena_string_capacity_zero() {
        // Arena strings don't use capacity field
        let s = arena_string("test");
        assert_eq!(s.capacity, 0); // Arena strings have capacity=0
        assert!(!s.is_global());
    }

    // ------------------------------------------------------------------
    // Byte-cleanliness sentinel tests.
    //
    // The type carries arbitrary bytes — no UTF-8 invariant. The
    // sentinel covers: a NUL byte, a non-UTF-8 lead byte (0xDC alone is
    // a UTF-8 continuation byte; standalone it's invalid), a high byte
    // (0xFF, never valid in any UTF-8 position), and a partial
    // multi-byte UTF-8 prefix (0xC3 without continuation). If any path
    // through the runtime mangles or rejects these, the bug shows up
    // here first.
    // ------------------------------------------------------------------

    const SENTINEL: &[u8] = &[0x00, 0xDC, b'x', 0xFF, 0xC3, b'!'];

    #[test]
    fn global_bytes_carries_arbitrary_bytes() {
        let s = global_bytes(SENTINEL.to_vec());
        assert_eq!(s.as_bytes(), SENTINEL);
        assert_eq!(s.len(), SENTINEL.len());
        assert!(s.is_global());
        // The sentinel isn't valid UTF-8, so as_str is None.
        assert_eq!(s.as_str(), None);
    }

    #[test]
    fn arena_bytes_carries_arbitrary_bytes() {
        let s = arena_bytes(SENTINEL);
        assert_eq!(s.as_bytes(), SENTINEL);
        assert_eq!(s.len(), SENTINEL.len());
        assert!(!s.is_global());
        assert_eq!(s.as_str(), None);
    }

    #[test]
    fn equality_uses_bytes_not_utf8() {
        // Two SeqStrings with identical non-UTF-8 bytes are equal.
        let s1 = arena_bytes(SENTINEL);
        let s2 = global_bytes(SENTINEL.to_vec());
        assert_eq!(s1, s2);

        // Differ in one byte.
        let mut alt = SENTINEL.to_vec();
        alt[0] = 0x01;
        let s3 = global_bytes(alt);
        assert_ne!(s1, s3);
    }

    #[test]
    fn clone_round_trips_arbitrary_bytes() {
        // Clone must preserve invalid UTF-8 byte-for-byte; it goes
        // through the global allocator (cross-strand transfer path).
        let s = arena_bytes(SENTINEL);
        let cloned = s.clone();
        assert_eq!(s.as_bytes(), cloned.as_bytes());
        assert!(cloned.is_global());
    }

    #[test]
    fn drop_does_not_require_utf8() {
        // Allocate-and-drop a global non-UTF-8 buffer. Pre-fix this
        // would be UB inside the Drop impl (String::from_raw_parts on
        // invalid UTF-8). The fixed Drop reconstructs a Vec<u8>
        // instead, which has no UTF-8 requirement.
        for _ in 0..16 {
            let _ = global_bytes(SENTINEL.to_vec());
        }
        // If we reach here without the allocator complaining, the
        // capacity bookkeeping is also intact for byte buffers.
    }

    #[test]
    fn as_str_lossy_replaces_invalid() {
        // Display path: invalid UTF-8 becomes U+FFFD, but the call
        // doesn't fail or panic.
        let s = global_bytes(SENTINEL.to_vec());
        let lossy = s.as_str_lossy();
        assert!(lossy.contains('\u{FFFD}'));
        // The valid 'x' and '!' bytes are still there.
        assert!(lossy.contains('x'));
        assert!(lossy.contains('!'));
    }
}
