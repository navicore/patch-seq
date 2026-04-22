use super::*;

#[test]
fn test_is_test_file() {
    let runner = TestRunner::new(false, None);
    assert!(runner.is_test_file(Path::new("test-foo.seq")));
    assert!(runner.is_test_file(Path::new("test-arithmetic.seq")));
    assert!(!runner.is_test_file(Path::new("foo.seq")));
    assert!(!runner.is_test_file(Path::new("test-foo.txt")));
    assert!(!runner.is_test_file(Path::new("my-test.seq")));
}

#[test]
fn test_discover_test_functions() {
    let runner = TestRunner::new(false, None);
    let source = r#"
: test-addition ( -- )
  2 3 add 5 test.assert-eq
;

: test-subtraction ( -- )
  5 3 subtract 2 test.assert-eq
;

: helper ( -- Int )
  42
;
"#;
    let (tests, has_main) = runner.discover_test_functions(source).unwrap();
    assert_eq!(tests.len(), 2);
    assert!(tests.contains(&"test-addition".to_string()));
    assert!(tests.contains(&"test-subtraction".to_string()));
    assert!(!tests.contains(&"helper".to_string()));
    assert!(!has_main);
}

#[test]
fn test_discover_with_main() {
    let runner = TestRunner::new(false, None);
    let source = r#"
: test-foo ( -- ) ;
: main ( -- ) ;
"#;
    let (tests, has_main) = runner.discover_test_functions(source).unwrap();
    assert_eq!(tests.len(), 1);
    assert!(has_main);
}

#[test]
fn test_filter() {
    let runner = TestRunner::new(false, Some("add".to_string()));
    let source = r#"
: test-addition ( -- ) ;
: test-subtraction ( -- ) ;
"#;
    let (tests, _) = runner.discover_test_functions(source).unwrap();
    assert_eq!(tests.len(), 1);
    assert!(tests.contains(&"test-addition".to_string()));
}

#[test]
fn test_sanitize_name() {
    assert_eq!(sanitize_name("test-foo"), "test_foo");
    assert_eq!(sanitize_name("test-foo-bar"), "test_foo_bar");
}
