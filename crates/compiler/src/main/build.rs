//! `seqc build` subcommand: compile a .seq file to an executable.

use std::path::{Path, PathBuf};
use std::process;

pub(crate) fn run_build(
    input: &Path,
    output: &Path,
    keep_ir: bool,
    ffi_manifests: &[PathBuf],
    pure_inline: bool,
    instrument: bool,
) {
    // Build config with external FFI manifests
    let mut config = if ffi_manifests.is_empty() {
        seqc::CompilerConfig::default()
    } else {
        seqc::CompilerConfig::new().with_ffi_manifests(ffi_manifests.iter().cloned())
    };

    // Enable pure inline test mode if requested
    config.pure_inline_test = pure_inline;

    // Enable per-word instrumentation if requested
    config.instrument = instrument;

    match seqc::compile_file_with_config(input, output, keep_ir, &config) {
        Ok(_) => {
            println!("Compiled {} -> {}", input.display(), output.display());

            if keep_ir {
                let ir_path = output.with_extension("ll");
                if ir_path.exists() {
                    println!("IR saved to {}", ir_path.display());
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
