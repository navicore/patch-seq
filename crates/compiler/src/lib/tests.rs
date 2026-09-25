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

#[test]
fn test_runtime_archive_selection_by_capability_words() {
    // io-only program: no capability words -> base archive
    let hello = ": main ( -- )  \"Hello, World!\" io.write-line ;";
    let program = Parser::new(hello).parse().unwrap();
    assert!(
        !program_needs_full_runtime(&program),
        "io-only program must select the base runtime"
    );

    // each capability namespace selects full
    for src in [
        ": main ( -- )  \"https://x\" net.http.get drop ;",
        ": main ( -- )  0 0 net.tls.client drop ;",
        ": main ( -- )  \"a\" crypto.sha256 drop ;",
        ": main ( -- )  \"a\" \"b\" regex.match? drop ;",
        ": main ( -- )  \"a\" compress.zstd drop ;",
    ] {
        let program = Parser::new(src).parse().unwrap();
        assert!(
            program_needs_full_runtime(&program),
            "capability word must select the full runtime: {src}"
        );
    }

    // capability word in an *unreachable* word still selects full
    // (conservative by design)
    let dead = ": unused ( -- )  \"a\" crypto.sha256 drop ;\n: main ( -- )  1 drop ;";
    let program = Parser::new(dead).parse().unwrap();
    assert!(program_needs_full_runtime(&program));
}
