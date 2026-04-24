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

#[test]
fn collect_failure_block_captures_indented_detail() {
    let output = "\
test-foo ... FAILED
  at line 7: expected 1, got 2
  +1 more failure
other-output
";
    let block = collect_failure_block(output, "test-foo").unwrap();
    assert_eq!(
        block,
        "test-foo ... FAILED\n  at line 7: expected 1, got 2\n  +1 more failure"
    );
}

#[test]
fn collect_failure_block_only_returns_target_block_when_adjacent() {
    let output = "\
test-one ... FAILED
  at line 1: expected 1, got 2
test-two ... FAILED
  at line 5: expected 3, got 4
";
    let one = collect_failure_block(output, "test-one").unwrap();
    let two = collect_failure_block(output, "test-two").unwrap();
    assert_eq!(one, "test-one ... FAILED\n  at line 1: expected 1, got 2");
    assert_eq!(two, "test-two ... FAILED\n  at line 5: expected 3, got 4");
}

#[test]
fn collect_failure_block_returns_none_when_absent() {
    let output = "\
test-foo ... ok
test-bar ... ok
";
    assert!(collect_failure_block(output, "test-foo").is_none());
    assert!(collect_failure_block(output, "missing").is_none());
}

#[test]
fn collect_failure_block_rejects_substring_false_positive() {
    // `add` is a prefix of `add-overflow`. The exact-line match must
    // not attribute `add-overflow`'s FAILED line to `add`.
    let output = "\
add-overflow ... FAILED
  at line 9: expected 0, got 1
";
    assert!(collect_failure_block(output, "add").is_none());
    assert!(collect_failure_block(output, "add-overflow").is_some());
}
