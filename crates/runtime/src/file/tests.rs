use super::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_file_slurp() {
    // Create a temporary file with known contents
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "Hello, file!").unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.into()));
        let stack = patch_seq_file_slurp(stack);

        // file-slurp now returns (contents Bool)
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        let (_stack, value) = pop(stack);
        match value {
            Value::String(s) => assert_eq!(s.as_str_or_empty().trim(), "Hello, file!"),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_file_exists_true() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.into()));
        let stack = patch_seq_file_exists(stack);

        let (_stack, value) = pop(stack);
        assert_eq!(value, Value::Bool(true));
    }
}

#[test]
fn test_file_exists_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("/nonexistent/path/to/file.txt".into()));
        let stack = patch_seq_file_exists(stack);

        let (_stack, value) = pop(stack);
        assert_eq!(value, Value::Bool(false));
    }
}

#[test]
fn test_file_slurp_utf8() {
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "Hello, 世界! 🌍").unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.into()));
        let stack = patch_seq_file_slurp(stack);

        // file-slurp returns (contents Bool)
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        let (_stack, value) = pop(stack);
        match value {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "Hello, 世界! 🌍"),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_file_slurp_empty() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.into()));
        let stack = patch_seq_file_slurp(stack);

        // file-slurp returns (contents Bool)
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true)); // Empty file is still success
        let (_stack, value) = pop(stack);
        match value {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), ""),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_file_slurp_not_found() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("/nonexistent/path/to/file.txt".into()));
        let stack = patch_seq_file_slurp(stack);

        let (stack, success) = pop(stack);
        let (_stack, contents) = pop(stack);
        assert_eq!(success, Value::Bool(false));
        match contents {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), ""),
            _ => panic!("Expected String"),
        }
    }
}

// ==========================================================================
// Tests for file.spit
// ==========================================================================

#[test]
fn test_file_spit_creates_new_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("test.txt");
    let path_str = path.to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("hello world".into()));
        let stack = push(stack, Value::String(path_str.clone().into()));
        let stack = patch_seq_file_spit(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
    }

    // Verify file was created with correct contents
    let contents = std::fs::read_to_string(&path).unwrap();
    assert_eq!(contents, "hello world");
}

#[test]
fn test_file_spit_overwrites_existing() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "old content").unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("new content".into()));
        let stack = push(stack, Value::String(path.clone().into()));
        let stack = patch_seq_file_spit(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
    }

    let contents = std::fs::read_to_string(&path).unwrap();
    assert_eq!(contents, "new content");
}

#[test]
fn test_file_spit_invalid_path() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("content".into()));
        let stack = push(stack, Value::String("/nonexistent/dir/file.txt".into()));
        let stack = patch_seq_file_spit(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
    }
}

// ==========================================================================
// Tests for file.append
// ==========================================================================

#[test]
fn test_file_append_to_existing() {
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "hello").unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(" world".into()));
        let stack = push(stack, Value::String(path.clone().into()));
        let stack = patch_seq_file_append(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
    }

    let contents = std::fs::read_to_string(&path).unwrap();
    assert_eq!(contents, "hello world");
}

#[test]
fn test_file_append_creates_new() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("new.txt");
    let path_str = path.to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("content".into()));
        let stack = push(stack, Value::String(path_str.clone().into()));
        let stack = patch_seq_file_append(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
    }

    let contents = std::fs::read_to_string(&path).unwrap();
    assert_eq!(contents, "content");
}

// ==========================================================================
// Tests for file.delete
// ==========================================================================

#[test]
fn test_file_delete_existing() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    // Keep path but drop temp_file so we control the file
    let path_copy = path.clone();
    drop(temp_file);
    std::fs::write(&path_copy, "content").unwrap();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path_copy.clone().into()));
        let stack = patch_seq_file_delete(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
    }

    assert!(!std::path::Path::new(&path_copy).exists());
}

#[test]
fn test_file_delete_nonexistent() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("/nonexistent/file.txt".into()));
        let stack = patch_seq_file_delete(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
    }
}

// ==========================================================================
// Tests for file.size
// ==========================================================================

#[test]
fn test_file_size_existing() {
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "hello world").unwrap(); // 11 bytes
    let path = temp_file.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.into()));
        let stack = patch_seq_file_size(stack);

        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        let (_stack, size) = pop(stack);
        assert_eq!(size, Value::Int(11));
    }
}

#[test]
fn test_file_size_nonexistent() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("/nonexistent/file.txt".into()));
        let stack = patch_seq_file_size(stack);

        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
        let (_stack, size) = pop(stack);
        assert_eq!(size, Value::Int(0));
    }
}

// ==========================================================================
// Tests for dir.exists?
// ==========================================================================

#[test]
fn test_dir_exists_true() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.into()));
        let stack = patch_seq_dir_exists(stack);

        let (_stack, exists) = pop(stack);
        assert_eq!(exists, Value::Bool(true));
    }
}

#[test]
fn test_dir_exists_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("/nonexistent/directory".into()));
        let stack = patch_seq_dir_exists(stack);

        let (_stack, exists) = pop(stack);
        assert_eq!(exists, Value::Bool(false));
    }
}

#[test]
fn test_dir_exists_file_is_not_dir() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.into()));
        let stack = patch_seq_dir_exists(stack);

        let (_stack, exists) = pop(stack);
        assert_eq!(exists, Value::Bool(false)); // file is not a directory
    }
}

// ==========================================================================
// Tests for dir.make
// ==========================================================================

#[test]
fn test_dir_make_success() {
    let temp_dir = tempfile::tempdir().unwrap();
    let new_dir = temp_dir.path().join("newdir");
    let path = new_dir.to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.clone().into()));
        let stack = patch_seq_dir_make(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
    }

    assert!(new_dir.is_dir());
}

#[test]
fn test_dir_make_nested() {
    let temp_dir = tempfile::tempdir().unwrap();
    let nested = temp_dir.path().join("a").join("b").join("c");
    let path = nested.to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.clone().into()));
        let stack = patch_seq_dir_make(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
    }

    assert!(nested.is_dir());
}

// ==========================================================================
// Tests for dir.delete
// ==========================================================================

#[test]
fn test_dir_delete_empty() {
    let temp_dir = tempfile::tempdir().unwrap();
    let to_delete = temp_dir.path().join("to_delete");
    std::fs::create_dir(&to_delete).unwrap();
    let path = to_delete.to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.clone().into()));
        let stack = patch_seq_dir_delete(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
    }

    assert!(!to_delete.exists());
}

#[test]
fn test_dir_delete_nonempty_fails() {
    let temp_dir = tempfile::tempdir().unwrap();
    let to_delete = temp_dir.path().join("nonempty");
    std::fs::create_dir(&to_delete).unwrap();
    std::fs::write(to_delete.join("file.txt"), "content").unwrap();
    let path = to_delete.to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.clone().into()));
        let stack = patch_seq_dir_delete(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false)); // can't delete non-empty
    }

    assert!(to_delete.exists());
}

#[test]
fn test_dir_delete_nonexistent() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("/nonexistent/dir".into()));
        let stack = patch_seq_dir_delete(stack);

        let (_stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
    }
}

// ==========================================================================
// Tests for dir.list
// ==========================================================================

#[test]
fn test_dir_list_success() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::fs::write(temp_dir.path().join("a.txt"), "a").unwrap();
    std::fs::write(temp_dir.path().join("b.txt"), "b").unwrap();
    let path = temp_dir.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.into()));
        let stack = patch_seq_dir_list(stack);

        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));

        let (_stack, list) = pop(stack);
        match list {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str_or_empty(), "List");
                assert_eq!(v.fields.len(), 2);
            }
            _ => panic!("Expected Variant(List)"),
        }
    }
}

#[test]
fn test_dir_list_empty() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap().to_string();

    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(path.into()));
        let stack = patch_seq_dir_list(stack);

        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));

        let (_stack, list) = pop(stack);
        match list {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str_or_empty(), "List");
                assert_eq!(v.fields.len(), 0);
            }
            _ => panic!("Expected Variant(List)"),
        }
    }
}

#[test]
fn test_dir_list_nonexistent() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("/nonexistent/dir".into()));
        let stack = patch_seq_dir_list(stack);

        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));

        let (_stack, list) = pop(stack);
        match list {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str_or_empty(), "List");
                assert_eq!(v.fields.len(), 0); // empty list on failure
            }
            _ => panic!("Expected Variant(List)"),
        }
    }
}
