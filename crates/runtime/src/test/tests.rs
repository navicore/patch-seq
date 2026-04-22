use super::*;

#[test]
fn test_context_reset() {
    let mut ctx = TestContext::new();
    ctx.record_pass();
    ctx.record_failure("test".to_string(), None, None);

    assert_eq!(ctx.passes, 1);
    assert_eq!(ctx.failures.len(), 1);

    ctx.reset(Some("new-test".to_string()));

    assert_eq!(ctx.passes, 0);
    assert!(ctx.failures.is_empty());
    assert_eq!(ctx.current_test, Some("new-test".to_string()));
}

#[test]
fn test_context_has_failures() {
    let mut ctx = TestContext::new();
    assert!(!ctx.has_failures());

    ctx.record_failure("error".to_string(), None, None);
    assert!(ctx.has_failures());
}
