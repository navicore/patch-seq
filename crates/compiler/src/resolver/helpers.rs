//! Free-standing helpers used alongside the `Resolver`: cross-file
//! collision detection for word and union names, plus the stdlib
//! discovery routine.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::ast::{SourceLocation, UnionDef, WordDef};

pub fn check_collisions(words: &[WordDef]) -> Result<(), String> {
    let mut definitions: HashMap<&str, Vec<&SourceLocation>> = HashMap::new();

    for word in words {
        if let Some(ref source) = word.source {
            definitions.entry(&word.name).or_default().push(source);
        }
    }

    // Find collisions (words defined in multiple places)
    let mut errors = Vec::new();
    for (name, locations) in definitions {
        if locations.len() > 1 {
            let mut msg = format!("Word '{}' is defined multiple times:\n", name);
            for loc in &locations {
                msg.push_str(&format!("  - {}\n", loc));
            }
            msg.push_str("\nHint: Rename one of the definitions to avoid collision.");
            errors.push(msg);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n\n"))
    }
}

/// Check for union name collisions across all definitions
///
/// Returns an error with helpful message if any union is defined multiple times.
pub fn check_union_collisions(unions: &[UnionDef]) -> Result<(), String> {
    let mut definitions: HashMap<&str, Vec<&SourceLocation>> = HashMap::new();

    for union_def in unions {
        if let Some(ref source) = union_def.source {
            definitions.entry(&union_def.name).or_default().push(source);
        }
    }

    // Find collisions (unions defined in multiple places)
    let mut errors = Vec::new();
    for (name, locations) in definitions {
        if locations.len() > 1 {
            let mut msg = format!("Union '{}' is defined multiple times:\n", name);
            for loc in &locations {
                msg.push_str(&format!("  - {}\n", loc));
            }
            msg.push_str("\nHint: Rename one of the definitions to avoid collision.");
            errors.push(msg);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n\n"))
    }
}

/// Find the stdlib directory for filesystem fallback
///
/// Searches in order:
/// 1. SEQ_STDLIB environment variable
/// 2. Relative to the current executable (for installed compilers)
/// 3. Relative to current directory (for development)
///
/// Returns None if no stdlib directory is found (embedded stdlib will be used).
pub fn find_stdlib() -> Option<PathBuf> {
    // Check environment variable first
    if let Ok(path) = std::env::var("SEQ_STDLIB") {
        let path = PathBuf::from(path);
        if path.is_dir() {
            return Some(path);
        }
        // If SEQ_STDLIB is set but invalid, log warning but continue
        eprintln!(
            "Warning: SEQ_STDLIB is set to '{}' but that directory doesn't exist",
            path.display()
        );
    }

    // Check relative to executable
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        let stdlib_path = exe_dir.join("stdlib");
        if stdlib_path.is_dir() {
            return Some(stdlib_path);
        }
        // Also check one level up (for development builds)
        if let Some(parent) = exe_dir.parent() {
            let stdlib_path = parent.join("stdlib");
            if stdlib_path.is_dir() {
                return Some(stdlib_path);
            }
        }
    }

    // Check relative to current directory (development)
    let local_stdlib = PathBuf::from("stdlib");
    if local_stdlib.is_dir() {
        return Some(local_stdlib.canonicalize().unwrap_or(local_stdlib));
    }

    // No filesystem stdlib found - that's OK, we have embedded stdlib
    None
}
