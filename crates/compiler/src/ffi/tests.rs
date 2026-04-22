use super::manifest::parse_stack_effect;
use super::*;
use crate::types::{StackType, Type};

#[test]
fn test_parse_manifest() {
    let content = r#"
[[library]]
name = "example"
link = "example"

[[library.function]]
c_name = "example_func"
seq_name = "example-func"
stack_effect = "( String -- String )"
args = [
  { type = "string", pass = "c_string" }
]
return = { type = "string", ownership = "caller_frees" }
"#;

    let manifest = FfiManifest::parse(content).unwrap();
    assert_eq!(manifest.libraries.len(), 1);
    assert_eq!(manifest.libraries[0].name, "example");
    assert_eq!(manifest.libraries[0].link, "example");
    assert_eq!(manifest.libraries[0].functions.len(), 1);

    let func = &manifest.libraries[0].functions[0];
    assert_eq!(func.c_name, "example_func");
    assert_eq!(func.seq_name, "example-func");
    assert_eq!(func.args.len(), 1);
    assert_eq!(func.args[0].arg_type, FfiType::String);
    assert_eq!(func.args[0].pass, PassMode::CString);
}

#[test]
fn test_parse_stack_effect() {
    let effect = parse_stack_effect("( String -- String )").unwrap();
    // Input: ( ..a String )
    let (rest, top) = effect.inputs.clone().pop().unwrap();
    assert_eq!(top, Type::String);
    assert_eq!(rest, StackType::RowVar("a".to_string()));
    // Output: ( ..a String )
    let (rest, top) = effect.outputs.clone().pop().unwrap();
    assert_eq!(top, Type::String);
    assert_eq!(rest, StackType::RowVar("a".to_string()));
}

#[test]
fn test_parse_stack_effect_void() {
    let effect = parse_stack_effect("( String -- )").unwrap();
    // Input: ( ..a String )
    let (rest, top) = effect.inputs.clone().pop().unwrap();
    assert_eq!(top, Type::String);
    assert_eq!(rest, StackType::RowVar("a".to_string()));
    // Output: ( ..a )
    assert_eq!(effect.outputs, StackType::RowVar("a".to_string()));
}

#[test]
fn test_ffi_bindings() {
    let content = r#"
[[library]]
name = "example"
link = "example"

[[library.function]]
c_name = "example_read"
seq_name = "example-read"
stack_effect = "( String -- String )"
args = [{ type = "string", pass = "c_string" }]
return = { type = "string", ownership = "caller_frees" }

[[library.function]]
c_name = "example_store"
seq_name = "example-store"
stack_effect = "( String -- )"
args = [{ type = "string", pass = "c_string" }]
return = { type = "void" }
"#;

    let manifest = FfiManifest::parse(content).unwrap();
    let mut bindings = FfiBindings::new();
    bindings.add_manifest(&manifest).unwrap();

    assert!(bindings.is_ffi_function("example-read"));
    assert!(bindings.is_ffi_function("example-store"));
    assert!(!bindings.is_ffi_function("not-defined"));

    assert_eq!(bindings.linker_flags, vec!["example"]);
}

// Validation tests

#[test]
fn test_validate_empty_library_name() {
    let content = r#"
[[library]]
name = ""
link = "example"

[[library.function]]
c_name = "example_func"
seq_name = "example-func"
stack_effect = "( String -- String )"
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty name"));
}

#[test]
fn test_validate_empty_link() {
    let content = r#"
[[library]]
name = "example"
link = "  "

[[library.function]]
c_name = "example_func"
seq_name = "example-func"
stack_effect = "( String -- String )"
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty linker flag"));
}

#[test]
fn test_validate_empty_c_name() {
    let content = r#"
[[library]]
name = "mylib"
link = "mylib"

[[library.function]]
c_name = ""
seq_name = "my-func"
stack_effect = "( -- Int )"
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty c_name"));
}

#[test]
fn test_validate_empty_seq_name() {
    let content = r#"
[[library]]
name = "mylib"
link = "mylib"

[[library.function]]
c_name = "my_func"
seq_name = ""
stack_effect = "( -- Int )"
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty seq_name"));
}

#[test]
fn test_validate_empty_stack_effect() {
    let content = r#"
[[library]]
name = "mylib"
link = "mylib"

[[library.function]]
c_name = "my_func"
seq_name = "my-func"
stack_effect = ""
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty stack_effect"));
}

#[test]
fn test_validate_malformed_stack_effect_no_parens() {
    let content = r#"
[[library]]
name = "mylib"
link = "mylib"

[[library.function]]
c_name = "my_func"
seq_name = "my-func"
stack_effect = "String -- Int"
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("malformed stack_effect"));
}

#[test]
fn test_validate_malformed_stack_effect_no_separator() {
    let content = r#"
[[library]]
name = "mylib"
link = "mylib"

[[library.function]]
c_name = "my_func"
seq_name = "my-func"
stack_effect = "( String Int )"
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("malformed stack_effect"));
    assert!(err.contains("--"));
}

#[test]
fn test_validate_malformed_stack_effect_unknown_type() {
    let content = r#"
[[library]]
name = "mylib"
link = "mylib"

[[library.function]]
c_name = "my_func"
seq_name = "my-func"
stack_effect = "( UnknownType -- Int )"
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("malformed stack_effect"));
    assert!(err.contains("Unknown type"));
}

#[test]
fn test_validate_no_libraries() {
    // TOML requires the `library` field to be present since it's not marked with #[serde(default)]
    // An empty manifest will fail TOML parsing, not our custom validation
    // But we can test with an explicit empty array
    let content = r#"
library = []
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("at least one library"));
}

#[test]
fn test_validate_linker_flag_injection() {
    // Security: reject linker flags with potentially dangerous characters
    let content = r#"
[[library]]
name = "evil"
link = "evil -Wl,-rpath,/malicious"

[[library.function]]
c_name = "func"
seq_name = "func"
stack_effect = "( -- )"
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("invalid character"));
}

#[test]
fn test_validate_linker_flag_valid() {
    // Valid linker flags: alphanumeric, dash, underscore, dot
    let content = r#"
[[library]]
name = "test"
link = "my-lib_2.0"

[[library.function]]
c_name = "func"
seq_name = "func"
stack_effect = "( -- )"
"#;

    let result = FfiManifest::parse(content);
    assert!(result.is_ok());
}
