use super::*;
use serial_test::serial;

#[test]
#[serial]
fn test_get_cache_dir_with_xdg() {
    // Save original env vars
    let orig_xdg = std::env::var("XDG_CACHE_HOME").ok();
    let orig_home = std::env::var("HOME").ok();

    // SAFETY: These tests must run serially (use cargo test -- --test-threads=1)
    // to avoid race conditions with other tests modifying environment variables.
    unsafe {
        // Test with XDG_CACHE_HOME set
        std::env::set_var("XDG_CACHE_HOME", "/tmp/test-xdg-cache");
    }
    let cache_dir = get_cache_dir();
    assert!(cache_dir.is_some());
    assert_eq!(cache_dir.unwrap(), PathBuf::from("/tmp/test-xdg-cache/seq"));

    // Restore original env vars
    // SAFETY: Restoring environment to original state
    unsafe {
        match orig_xdg {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        match orig_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
    }
}

#[test]
#[serial]
fn test_get_cache_dir_fallback_to_home() {
    // Save original env vars
    let orig_xdg = std::env::var("XDG_CACHE_HOME").ok();
    let orig_home = std::env::var("HOME").ok();

    // SAFETY: These tests must run serially (use cargo test -- --test-threads=1)
    // to avoid race conditions with other tests modifying environment variables.
    unsafe {
        // Clear XDG_CACHE_HOME, set HOME
        std::env::remove_var("XDG_CACHE_HOME");
        std::env::set_var("HOME", "/tmp/test-home");
    }
    let cache_dir = get_cache_dir();
    assert!(cache_dir.is_some());
    assert_eq!(
        cache_dir.unwrap(),
        PathBuf::from("/tmp/test-home/.cache/seq")
    );

    // Restore original env vars
    // SAFETY: Restoring environment to original state
    unsafe {
        match orig_xdg {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        match orig_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
    }
}

#[test]
fn test_compute_cache_key_deterministic() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let source = temp.path().join("test.seq");
    fs::write(&source, ": main ( -- Int ) 42 ;").unwrap();

    let key1 = compute_cache_key(&source, std::slice::from_ref(&source), &[]).unwrap();
    let key2 = compute_cache_key(&source, std::slice::from_ref(&source), &[]).unwrap();

    assert_eq!(key1, key2);
    assert_eq!(key1.len(), 64); // SHA-256 hex is 64 chars
}

#[test]
fn test_compute_cache_key_changes_with_content() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let source = temp.path().join("test.seq");

    fs::write(&source, ": main ( -- Int ) 42 ;").unwrap();
    let key1 = compute_cache_key(&source, std::slice::from_ref(&source), &[]).unwrap();

    fs::write(&source, ": main ( -- Int ) 43 ;").unwrap();
    let key2 = compute_cache_key(&source, std::slice::from_ref(&source), &[]).unwrap();

    assert_ne!(key1, key2);
}

#[test]
fn test_compute_cache_key_includes_embedded_modules() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let source = temp.path().join("test.seq");
    fs::write(&source, ": main ( -- Int ) 42 ;").unwrap();

    let key1 = compute_cache_key(&source, std::slice::from_ref(&source), &[]).unwrap();
    let key2 = compute_cache_key(
        &source,
        std::slice::from_ref(&source),
        &["imath".to_string()],
    )
    .unwrap();

    assert_ne!(key1, key2);
}

#[test]
fn test_strip_shebang_with_shebang() {
    let source = "#!/usr/bin/env seqc\n: main ( -- Int ) 42 ;";
    let stripped = strip_shebang(source);
    // Should start with # (comment) not #!
    assert!(stripped.starts_with('#'));
    assert!(!stripped.starts_with("#!"));
    // Should preserve the second line
    assert!(stripped.contains(": main ( -- Int ) 42 ;"));
    // Should preserve line count (same length before newline)
    assert_eq!(stripped.matches('\n').count(), source.matches('\n').count());
}

#[test]
fn test_strip_shebang_without_shebang() {
    let source = ": main ( -- Int ) 42 ;";
    let stripped = strip_shebang(source);
    // Should be unchanged
    assert_eq!(stripped.as_ref(), source);
}

#[test]
fn test_strip_shebang_with_comment() {
    let source = "# This is a comment\n: main ( -- Int ) 42 ;";
    let stripped = strip_shebang(source);
    // Should be unchanged (# is not #!)
    assert_eq!(stripped.as_ref(), source);
}

#[test]
fn test_strip_shebang_only_shebang() {
    let source = "#!/usr/bin/env seqc";
    let stripped = strip_shebang(source);
    // Single line file with just shebang becomes just #
    assert_eq!(stripped.as_ref(), "#");
}
