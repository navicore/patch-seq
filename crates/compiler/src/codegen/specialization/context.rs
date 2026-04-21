//! `RegisterContext` — the SSA-variable stack used while generating
//! specialized code. Unlike the memory-based tagged stack, this tracks
//! SSA variable names that hold values directly in registers, so the
//! Forth-style stack shuffles (`dup`, `swap`, `rot`, …) become free
//! context manipulations rather than load/store sequences.

use super::types::RegisterType;

/// Tracks values during specialized code generation.
#[derive(Debug, Clone)]
pub struct RegisterContext {
    /// Stack of (ssa_var_name, register_type) pairs, bottom to top
    pub values: Vec<(String, RegisterType)>,
}

impl RegisterContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Create a context initialized with function parameters
    pub fn from_params(params: &[(String, RegisterType)]) -> Self {
        Self {
            values: params.to_vec(),
        }
    }

    /// Push a value onto the register context
    pub fn push(&mut self, ssa_var: String, ty: RegisterType) {
        self.values.push((ssa_var, ty));
    }

    /// Pop a value from the register context
    pub fn pop(&mut self) -> Option<(String, RegisterType)> {
        self.values.pop()
    }

    /// Get the number of values in the context
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Perform dup: ( a -- a a ) - duplicate top value
    /// Note: For registers, this is a no-op at the SSA level,
    /// we just reference the same SSA variable twice
    pub fn dup(&mut self) {
        if let Some((ssa, ty)) = self.values.last().cloned() {
            self.values.push((ssa, ty));
        }
    }

    /// Perform drop: ( a -- )
    pub fn drop(&mut self) {
        self.values.pop();
    }

    /// Perform swap: ( a b -- b a )
    pub fn swap(&mut self) {
        let len = self.values.len();
        if len >= 2 {
            self.values.swap(len - 1, len - 2);
        }
    }

    /// Perform over: ( a b -- a b a )
    pub fn over(&mut self) {
        let len = self.values.len();
        if len >= 2 {
            let a = self.values[len - 2].clone();
            self.values.push(a);
        }
    }

    /// Perform rot: ( a b c -- b c a )
    pub fn rot(&mut self) {
        let len = self.values.len();
        if len >= 3 {
            let a = self.values.remove(len - 3);
            self.values.push(a);
        }
    }
}

impl Default for RegisterContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_context_stack_ops() {
        let mut ctx = RegisterContext::new();
        ctx.push("a".to_string(), RegisterType::I64);
        ctx.push("b".to_string(), RegisterType::I64);

        assert_eq!(ctx.len(), 2);

        // Test swap
        ctx.swap();
        assert_eq!(ctx.values[0].0, "b");
        assert_eq!(ctx.values[1].0, "a");

        // Test dup
        ctx.dup();
        assert_eq!(ctx.len(), 3);
        assert_eq!(ctx.values[2].0, "a");

        // Test drop
        ctx.drop();
        assert_eq!(ctx.len(), 2);
    }
}
