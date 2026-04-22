use super::*;
use crate::ast::{Include, SourceLocation, WordDef};
use crate::stdlib_embed;
use std::path::{Path, PathBuf};

#[test]
fn test_collision_detection_no_collision() {
    let words = vec![
        WordDef {
            name: "foo".to_string(),
            effect: None,
            body: vec![],
            source: Some(SourceLocation::new(PathBuf::from("a.seq"), 1)),
            allowed_lints: vec![],
        },
        WordDef {
            name: "bar".to_string(),
            effect: None,
            body: vec![],
            source: Some(SourceLocation::new(PathBuf::from("b.seq"), 1)),
            allowed_lints: vec![],
        },
    ];

    assert!(check_collisions(&words).is_ok());
}

#[test]
fn test_collision_detection_with_collision() {
    let words = vec![
        WordDef {
            name: "foo".to_string(),
            effect: None,
            body: vec![],
            source: Some(SourceLocation::new(PathBuf::from("a.seq"), 1)),
            allowed_lints: vec![],
        },
        WordDef {
            name: "foo".to_string(),
            effect: None,
            body: vec![],
            source: Some(SourceLocation::new(PathBuf::from("b.seq"), 5)),
            allowed_lints: vec![],
        },
    ];

    let result = check_collisions(&words);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("foo"));
    assert!(error.contains("a.seq"));
    assert!(error.contains("b.seq"));
    assert!(error.contains("multiple times"));
}

#[test]
fn test_collision_detection_same_file_different_lines() {
    // Same word defined twice in same file on different lines
    // This is still a collision (parser would typically catch this earlier)
    let words = vec![
        WordDef {
            name: "foo".to_string(),
            effect: None,
            body: vec![],
            source: Some(SourceLocation::new(PathBuf::from("a.seq"), 1)),
            allowed_lints: vec![],
        },
        WordDef {
            name: "foo".to_string(),
            effect: None,
            body: vec![],
            source: Some(SourceLocation::new(PathBuf::from("a.seq"), 5)),
            allowed_lints: vec![],
        },
    ];

    // This IS a collision - same name defined twice
    let result = check_collisions(&words);
    assert!(result.is_err());
}

// Integration tests for embedded stdlib

#[test]
fn test_embedded_stdlib_imath_available() {
    assert!(stdlib_embed::has_stdlib("imath"));
}

#[test]
fn test_embedded_stdlib_resolution() {
    let resolver = Resolver::new(None);
    let include = Include::Std("imath".to_string());
    let result = resolver.resolve_include(&include, Path::new("."));
    assert!(result.is_ok());
    match result.unwrap() {
        ResolvedInclude::Embedded(name, content) => {
            assert_eq!(name, "imath");
            assert!(content.contains("abs"));
        }
        ResolvedInclude::FilePath(_) => panic!("Expected embedded, got file path"),
    }
}

#[test]
fn test_nonexistent_stdlib_module() {
    let resolver = Resolver::new(None);
    let include = Include::Std("nonexistent".to_string());
    let result = resolver.resolve_include(&include, Path::new("."));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_resolver_with_no_stdlib_path() {
    // Resolver should work with None stdlib_path, using only embedded modules
    let resolver = Resolver::new(None);
    assert!(resolver.stdlib_path.is_none());
}

#[test]
fn test_double_include_prevention_embedded() {
    let mut resolver = Resolver::new(None);

    // First include should work
    let result1 = resolver.process_embedded_include(
        "imath",
        stdlib_embed::get_stdlib("imath").unwrap(),
        Path::new("."),
    );
    assert!(result1.is_ok());
    let content1 = result1.unwrap();
    assert!(!content1.words.is_empty());

    // Second include of same module should return empty (already included)
    let result2 = resolver.process_embedded_include(
        "imath",
        stdlib_embed::get_stdlib("imath").unwrap(),
        Path::new("."),
    );
    assert!(result2.is_ok());
    let content2 = result2.unwrap();
    assert!(content2.words.is_empty());
    assert!(content2.unions.is_empty());
}

#[test]
fn test_cross_directory_include_allowed() {
    // Test that ".." paths work for cross-directory includes
    use std::fs;
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let root = temp.path();

    // Create directory structure:
    // root/
    //   src/
    //     lib/
    //       helper.seq
    //   tests/
    //     test_main.seq (wants to include ../src/lib/helper)
    let src = root.join("src");
    let src_lib = src.join("lib");
    let tests = root.join("tests");
    fs::create_dir_all(&src_lib).unwrap();
    fs::create_dir_all(&tests).unwrap();

    // Create helper.seq in src/lib
    fs::write(src_lib.join("helper.seq"), ": helper ( -- Int ) 42 ;\n").unwrap();

    let resolver = Resolver::new(None);

    // Resolve from tests directory: include ../src/lib/helper
    let include = Include::Relative("../src/lib/helper".to_string());
    let result = resolver.resolve_include(&include, &tests);

    assert!(
        result.is_ok(),
        "Cross-directory include should succeed: {:?}",
        result.err()
    );

    match result.unwrap() {
        ResolvedInclude::FilePath(path) => {
            assert!(path.ends_with("helper.seq"));
        }
        ResolvedInclude::Embedded(_, _) => panic!("Expected file path, got embedded"),
    }
}

#[test]
fn test_dotdot_within_same_directory_structure() {
    // Test that "../../file" resolves correctly
    use std::fs;
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let project = temp.path();

    // Create: project/a/b/c/ and project/a/target.seq
    let deep = project.join("a").join("b").join("c");
    fs::create_dir_all(&deep).unwrap();
    fs::write(project.join("a").join("target.seq"), ": target ( -- ) ;\n").unwrap();

    let resolver = Resolver::new(None);

    // From a/b/c, include ../../target should work
    let include = Include::Relative("../../target".to_string());
    let result = resolver.resolve_include(&include, &deep);

    assert!(
        result.is_ok(),
        "Include with .. should work: {:?}",
        result.err()
    );
}

#[test]
fn test_empty_include_path_rejected() {
    let resolver = Resolver::new(None);
    let include = Include::Relative("".to_string());
    let result = resolver.resolve_include(&include, Path::new("."));

    assert!(result.is_err(), "Empty include path should be rejected");
    assert!(
        result.unwrap_err().contains("cannot be empty"),
        "Error should mention empty path"
    );
}
