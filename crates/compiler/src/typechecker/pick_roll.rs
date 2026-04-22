//! Specialized handlers for constant-N pick/roll during statement checking.

use crate::types::StackType;
use crate::unification::Subst;

use super::TypeChecker;

impl TypeChecker {
    pub(super) fn handle_literal_pick(
        &self,
        n: i64,
        current_stack: StackType,
    ) -> Result<(StackType, Subst), String> {
        if n < 0 {
            return Err(format!("pick: index must be non-negative, got {}", n));
        }

        // Get the type at position n
        let type_at_n = self.get_type_at_position(&current_stack, n as usize, "pick")?;

        // Push a copy of that type onto the stack
        Ok((current_stack.push(type_at_n), Subst::empty()))
    }

    /// Handle `n roll` where n is a literal integer
    ///
    /// roll(n) moves the value at position n to the top of the stack,
    /// shifting all items above it down by one position.
    ///
    /// Example: `2 roll` on stack ( A B C ) produces ( B C A )
    /// - Position 0: C (top)
    /// - Position 1: B
    /// - Position 2: A
    /// - Result: move A to top, B and C shift down
    pub(super) fn handle_literal_roll(
        &self,
        n: i64,
        current_stack: StackType,
    ) -> Result<(StackType, Subst), String> {
        if n < 0 {
            return Err(format!("roll: index must be non-negative, got {}", n));
        }

        // For roll, we need to:
        // 1. Extract the type at position n
        // 2. Remove it from that position
        // 3. Push it on top
        self.rotate_type_to_top(current_stack, n as usize)
    }
}
