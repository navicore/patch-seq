//! Compiler Bridge for TUI
//!
//! Provides IR extraction by wrapping seq-compiler functionality.
//! Converts compiler types to TUI-friendly representations.

use seqc::{CodeGen, CompilerConfig, Parser, TypeChecker};

/// Result of analyzing Seq source code
#[derive(Debug, Clone)]
pub(crate) struct AnalysisResult {
    /// Any errors encountered during analysis
    pub(crate) errors: Vec<String>,
    /// LLVM IR if compilation succeeded
    pub(crate) llvm_ir: Option<String>,
}

/// Analyze Seq source code and extract IR information
pub(crate) fn analyze(source: &str) -> AnalysisResult {
    analyze_with_config(source, &CompilerConfig::default())
}

/// Analyze a standalone expression for IR display
/// Wraps the expression in a minimal function to generate clean IR
pub(crate) fn analyze_expression(expr: &str) -> Option<Vec<String>> {
    // Skip empty or whitespace-only expressions
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }

    // Wrap expression in a standalone function with permissive stack effect
    // ( ..a -- ..b ) allows any stack transformation
    // Need a main word for codegen to work
    let source = format!(
        ": __expr__ ( ..a -- ..b )\n  {}\n;\n: main ( -- ) ;\n",
        expr
    );

    let result = analyze(&source);

    // Extract just the __expr__ function's IR (compiler adds seq_ prefix)
    if let Some(ir) = result.llvm_ir {
        let lines = extract_function_ir(&ir, "@seq___expr__");
        if !lines.is_empty() {
            return Some(lines);
        }
    }

    None
}

/// Extract a specific function's IR from LLVM output
fn extract_function_ir(ir: &str, func_name: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut in_function = false;

    for line in ir.lines() {
        if line.contains("define") && line.contains(func_name) {
            in_function = true;
        }
        if in_function {
            lines.push(line.to_string());
            if line.trim() == "}" {
                break;
            }
        }
    }

    lines
}

/// Analyze Seq source code with custom configuration
fn analyze_with_config(source: &str, config: &CompilerConfig) -> AnalysisResult {
    let mut errors = Vec::new();
    let mut llvm_ir = None;

    let mut parser = Parser::new(source);
    let mut program = match parser.parse() {
        Ok(prog) => prog,
        Err(e) => {
            errors.push(format!("Parse error: {}", e));
            return AnalysisResult { errors, llvm_ir };
        }
    };

    // Generate constructors for unions
    if !program.unions.is_empty()
        && let Err(e) = program.generate_constructors()
    {
        errors.push(format!("Constructor generation error: {}", e));
    }

    let mut typechecker = TypeChecker::new();

    // Register external builtins if configured
    // All external builtins must have explicit effects (v2.0 requirement)
    if !config.external_builtins.is_empty() {
        for builtin in &config.external_builtins {
            if builtin.effect.is_none() {
                errors.push(format!(
                    "External builtin '{}' is missing a stack effect declaration.",
                    builtin.seq_name
                ));
            }
        }
        let external_effects: Vec<_> = config
            .external_builtins
            .iter()
            .filter_map(|b| b.effect.as_ref().map(|e| (b.seq_name.as_str(), e)))
            .collect();
        typechecker.register_external_words(&external_effects);
    }

    if let Err(e) = typechecker.check_program(&program) {
        errors.push(format!("Type error: {}", e));
    }

    // Try to generate LLVM IR (only if no errors)
    if errors.is_empty() {
        let quotation_types = typechecker.take_quotation_types();
        let statement_types = typechecker.take_statement_top_types();
        let aux_max_depths = typechecker.take_aux_max_depths();
        let resolved_sugar = typechecker.take_resolved_sugar();
        let mut codegen = CodeGen::new();
        codegen.set_aux_slot_counts(aux_max_depths);
        codegen.set_resolved_sugar(resolved_sugar);
        match codegen.codegen_program_with_config(
            &program,
            quotation_types,
            statement_types,
            config,
        ) {
            Ok(ir) => llvm_ir = Some(ir),
            Err(e) => errors.push(format!("Codegen error: {}", e)),
        }
    }

    AnalysisResult { errors, llvm_ir }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_simple_program() {
        let source = r#"
: main ( -- )
    42 drop
;
"#;
        let result = analyze(source);
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert!(result.llvm_ir.is_some());
    }

    #[test]
    fn test_analyze_expression_simple() {
        // Test that analyze_expression works for simple literals
        let result = analyze_expression("5");
        assert!(result.is_some(), "Should produce IR for '5'");
        let ir = result.unwrap();
        assert!(ir.iter().any(|l| l.contains("seq___expr__")));

        // Test arithmetic expression
        let result = analyze_expression("5 10 i.add");
        assert!(result.is_some(), "Should produce IR for '5 10 i.add'");
    }

    #[test]
    fn test_analyze_type_error() {
        let source = r#"
: main ( -- )
    "hello" 42 i.add
;
"#;
        let result = analyze(source);
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].contains("error") || result.errors[0].contains("mismatch"));
    }
}
