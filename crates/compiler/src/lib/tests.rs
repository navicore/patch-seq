use super::*;

#[test]
fn test_parse_clang_version_standard() {
    let output = "clang version 15.0.0 (https://github.com/llvm/llvm-project)\nTarget: x86_64";
    assert_eq!(parse_clang_version(output), Some(15));
}

#[test]
fn test_parse_clang_version_apple() {
    let output = "Apple clang version 14.0.3 (clang-1403.0.22.14.1)\nTarget: arm64-apple-darwin";
    assert_eq!(parse_clang_version(output), Some(14));
}

#[test]
fn test_parse_clang_version_homebrew() {
    let output = "Homebrew clang version 17.0.6\nTarget: arm64-apple-darwin23.0.0";
    assert_eq!(parse_clang_version(output), Some(17));
}

#[test]
fn test_parse_clang_version_ubuntu() {
    let output = "Ubuntu clang version 15.0.7\nTarget: x86_64-pc-linux-gnu";
    assert_eq!(parse_clang_version(output), Some(15));
}

#[test]
fn test_parse_clang_version_invalid() {
    assert_eq!(parse_clang_version("no version here"), None);
    assert_eq!(parse_clang_version("version "), None);
}
