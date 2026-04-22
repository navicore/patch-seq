//! Call graph analysis for detecting mutual recursion
//!
//! This module builds a call graph from a Seq program and detects
//! strongly connected components (SCCs) to identify mutual recursion cycles.
//!
//! # Usage
//!
//! ```ignore
//! let call_graph = CallGraph::build(&program);
//! let cycles = call_graph.recursive_cycles();
//! ```
//!
//! # Primary Use Cases
//!
//! 1. **Type checker divergence detection**: The type checker uses the call graph
//!    to identify mutually recursive tail calls, enabling correct type inference
//!    for patterns like even/odd that would otherwise require branch unification.
//!
//! 2. **Future optimizations**: The call graph infrastructure can support dead code
//!    detection, inlining decisions, and diagnostic tools.
//!
//! # Implementation Details
//!
//! - **Algorithm**: Tarjan's SCC algorithm, O(V + E) time complexity
//! - **Builtins**: Calls to builtins/external words are excluded from the graph
//!   (they don't affect recursion detection since they always return)
//! - **Quotations**: Calls within quotations are included in the analysis
//! - **Match arms**: Calls within match arms are included in the analysis
//!
//! # Note on Tail Call Optimization
//!
//! The existing codegen already emits `musttail` for all tail calls to user-defined
//! words (see `codegen/statements.rs`). This means mutual TCO works automatically
//! without needing explicit call graph checks in codegen. The call graph is primarily
//! used for type checking, not for enabling TCO.

use crate::ast::{Program, Statement};
use std::collections::{HashMap, HashSet};

/// A call graph representing which words call which other words.
#[derive(Debug, Clone)]
pub struct CallGraph {
    /// Map from word name to the set of words it calls
    edges: HashMap<String, HashSet<String>>,
    /// All word names in the program
    words: HashSet<String>,
    /// Strongly connected components with more than one member (mutual recursion)
    /// or single members that call themselves (direct recursion)
    recursive_sccs: Vec<HashSet<String>>,
}

impl CallGraph {
    /// Build a call graph from a program.
    ///
    /// This extracts all word-to-word call relationships, including calls
    /// within quotations, if branches, and match arms.
    pub fn build(program: &Program) -> Self {
        let mut edges: HashMap<String, HashSet<String>> = HashMap::new();
        let words: HashSet<String> = program.words.iter().map(|w| w.name.clone()).collect();

        for word in &program.words {
            let callees = extract_calls(&word.body, &words);
            edges.insert(word.name.clone(), callees);
        }

        let mut graph = CallGraph {
            edges,
            words,
            recursive_sccs: Vec::new(),
        };

        // Compute SCCs and identify recursive cycles
        graph.recursive_sccs = graph.find_sccs();

        graph
    }

    /// Check if a word is part of any recursive cycle (direct or mutual).
    pub fn is_recursive(&self, word: &str) -> bool {
        self.recursive_sccs.iter().any(|scc| scc.contains(word))
    }

    /// Check if two words are in the same recursive cycle (mutually recursive).
    pub fn are_mutually_recursive(&self, word1: &str, word2: &str) -> bool {
        self.recursive_sccs
            .iter()
            .any(|scc| scc.contains(word1) && scc.contains(word2))
    }

    /// Get all recursive cycles (SCCs with recursion).
    pub fn recursive_cycles(&self) -> &[HashSet<String>] {
        &self.recursive_sccs
    }

    /// Get the words that a given word calls.
    pub fn callees(&self, word: &str) -> Option<&HashSet<String>> {
        self.edges.get(word)
    }

    /// Find strongly connected components using Tarjan's algorithm.
    ///
    /// Returns only SCCs that represent recursion:
    /// - Multi-word SCCs (mutual recursion)
    /// - Single-word SCCs where the word calls itself (direct recursion)
    fn find_sccs(&self) -> Vec<HashSet<String>> {
        let mut index_counter = 0;
        let mut stack: Vec<String> = Vec::new();
        let mut on_stack: HashSet<String> = HashSet::new();
        let mut indices: HashMap<String, usize> = HashMap::new();
        let mut lowlinks: HashMap<String, usize> = HashMap::new();
        let mut sccs: Vec<HashSet<String>> = Vec::new();

        for word in &self.words {
            if !indices.contains_key(word) {
                self.tarjan_visit(
                    word,
                    &mut index_counter,
                    &mut stack,
                    &mut on_stack,
                    &mut indices,
                    &mut lowlinks,
                    &mut sccs,
                );
            }
        }

        // Filter to only recursive SCCs
        sccs.into_iter()
            .filter(|scc| {
                if scc.len() > 1 {
                    // Multi-word SCC = mutual recursion
                    true
                } else if scc.len() == 1 {
                    // Single-word SCC: check if it calls itself
                    let word = scc.iter().next().unwrap();
                    self.edges
                        .get(word)
                        .map(|callees| callees.contains(word))
                        .unwrap_or(false)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Tarjan's algorithm recursive visit.
    #[allow(clippy::too_many_arguments)]
    fn tarjan_visit(
        &self,
        word: &str,
        index_counter: &mut usize,
        stack: &mut Vec<String>,
        on_stack: &mut HashSet<String>,
        indices: &mut HashMap<String, usize>,
        lowlinks: &mut HashMap<String, usize>,
        sccs: &mut Vec<HashSet<String>>,
    ) {
        let index = *index_counter;
        *index_counter += 1;
        indices.insert(word.to_string(), index);
        lowlinks.insert(word.to_string(), index);
        stack.push(word.to_string());
        on_stack.insert(word.to_string());

        // Visit all callees
        if let Some(callees) = self.edges.get(word) {
            for callee in callees {
                if !self.words.contains(callee) {
                    // External word (builtin), skip
                    continue;
                }
                if !indices.contains_key(callee) {
                    // Not yet visited
                    self.tarjan_visit(
                        callee,
                        index_counter,
                        stack,
                        on_stack,
                        indices,
                        lowlinks,
                        sccs,
                    );
                    let callee_lowlink = *lowlinks.get(callee).unwrap();
                    let word_lowlink = lowlinks.get_mut(word).unwrap();
                    *word_lowlink = (*word_lowlink).min(callee_lowlink);
                } else if on_stack.contains(callee) {
                    // Callee is on stack, part of current SCC
                    let callee_index = *indices.get(callee).unwrap();
                    let word_lowlink = lowlinks.get_mut(word).unwrap();
                    *word_lowlink = (*word_lowlink).min(callee_index);
                }
            }
        }

        // If word is a root node, pop the SCC
        if lowlinks.get(word) == indices.get(word) {
            let mut scc = HashSet::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.remove(&w);
                scc.insert(w.clone());
                if w == word {
                    break;
                }
            }
            sccs.push(scc);
        }
    }
}

/// Extract all word calls from a list of statements.
///
/// This recursively descends into quotations, if branches, and match arms.
fn extract_calls(statements: &[Statement], known_words: &HashSet<String>) -> HashSet<String> {
    let mut calls = HashSet::new();

    for stmt in statements {
        extract_calls_from_statement(stmt, known_words, &mut calls);
    }

    calls
}

/// Extract word calls from a single statement.
fn extract_calls_from_statement(
    stmt: &Statement,
    known_words: &HashSet<String>,
    calls: &mut HashSet<String>,
) {
    match stmt {
        Statement::WordCall { name, .. } => {
            // Only track calls to user-defined words
            if known_words.contains(name) {
                calls.insert(name.clone());
            }
        }
        Statement::If {
            then_branch,
            else_branch,
            span: _,
        } => {
            for s in then_branch {
                extract_calls_from_statement(s, known_words, calls);
            }
            if let Some(else_stmts) = else_branch {
                for s in else_stmts {
                    extract_calls_from_statement(s, known_words, calls);
                }
            }
        }
        Statement::Quotation { body, .. } => {
            for s in body {
                extract_calls_from_statement(s, known_words, calls);
            }
        }
        Statement::Match { arms, span: _ } => {
            for arm in arms {
                for s in &arm.body {
                    extract_calls_from_statement(s, known_words, calls);
                }
            }
        }
        // Literals don't contain calls
        Statement::IntLiteral(_)
        | Statement::FloatLiteral(_)
        | Statement::BoolLiteral(_)
        | Statement::StringLiteral(_)
        | Statement::Symbol(_) => {}
    }
}

#[cfg(test)]
mod tests;
