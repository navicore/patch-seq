#![allow(deprecated)]

use super::*;

#[test]
fn test_external_builtin_new() {
    let builtin = ExternalBuiltin::new("my-func", "runtime_my_func");
    assert_eq!(builtin.seq_name, "my-func");
    assert_eq!(builtin.symbol, "runtime_my_func");
    assert!(builtin.effect.is_none());
}

#[test]
fn test_config_builder() {
    let config = CompilerConfig::new()
        .with_builtin(ExternalBuiltin::new("func-a", "sym_a"))
        .with_builtin(ExternalBuiltin::new("func-b", "sym_b"))
        .with_library_path("/custom/lib")
        .with_library("myruntime");

    assert_eq!(config.external_builtins.len(), 2);
    assert_eq!(config.library_paths, vec!["/custom/lib"]);
    assert_eq!(config.libraries, vec!["myruntime"]);
}

#[test]
fn test_external_names() {
    let config = CompilerConfig::new()
        .with_builtin(ExternalBuiltin::new("func-a", "sym_a"))
        .with_builtin(ExternalBuiltin::new("func-b", "sym_b"));

    let names = config.external_names();
    assert_eq!(names, vec!["func-a", "func-b"]);
}

#[test]
fn test_symbol_validation_valid() {
    // Valid symbols: alphanumeric, underscores, periods
    let _ = ExternalBuiltin::new("test", "valid_symbol");
    let _ = ExternalBuiltin::new("test", "valid.symbol.123");
    let _ = ExternalBuiltin::new("test", "ValidCamelCase");
    let _ = ExternalBuiltin::new("test", "seq_actors_journal_append");
}

#[test]
#[should_panic(expected = "Invalid symbol name")]
fn test_symbol_validation_rejects_hyphen() {
    // Hyphens are not valid in LLVM symbols
    let _ = ExternalBuiltin::new("test", "invalid-symbol");
}

#[test]
#[should_panic(expected = "Invalid symbol name")]
fn test_symbol_validation_rejects_at() {
    // @ could be used for LLVM IR injection
    let _ = ExternalBuiltin::new("test", "@malicious");
}

#[test]
#[should_panic(expected = "Invalid symbol name")]
fn test_symbol_validation_rejects_empty() {
    let _ = ExternalBuiltin::new("test", "");
}
