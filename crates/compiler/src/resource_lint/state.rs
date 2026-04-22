//! Shared types for resource-leak analysis: the abstract stack simulator
//! (`StackState`), the tracked-resource representation, and per-word
//! resource-return info used by the program-wide analyzer.

use std::collections::HashMap;

/// Identifies a resource type for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ResourceKind {
    /// Weave handle from `strand.weave`
    WeaveHandle,
    /// Channel from `chan.make`
    Channel,
}

impl ResourceKind {
    pub(super) fn name(&self) -> &'static str {
        match self {
            ResourceKind::WeaveHandle => "WeaveHandle",
            ResourceKind::Channel => "Channel",
        }
    }

    pub(super) fn cleanup_suggestion(&self) -> &'static str {
        match self {
            ResourceKind::WeaveHandle => "use `strand.weave-cancel` or resume to completion",
            ResourceKind::Channel => "use `chan.close` when done",
        }
    }
}

/// A tracked resource with its origin
#[derive(Debug, Clone)]
pub(crate) struct TrackedResource {
    /// What kind of resource this is
    pub kind: ResourceKind,
    /// Unique ID for this resource instance
    pub id: usize,
    /// Line where the resource was created (0-indexed)
    pub created_line: usize,
    /// The word that created this resource
    pub created_by: String,
}

/// A value on the abstract stack - either a resource or unknown
#[derive(Debug, Clone)]
pub(crate) enum StackValue {
    /// A tracked resource
    Resource(TrackedResource),
    /// An unknown value (literal, result of non-resource operation)
    Unknown,
}

/// State of the abstract stack during analysis
#[derive(Debug, Clone)]
pub(crate) struct StackState {
    /// The stack contents (top is last element)
    pub(super) stack: Vec<StackValue>,
    /// Aux stack contents for >aux/aux> simulation (Issue #350)
    pub(super) aux_stack: Vec<StackValue>,
    /// Resources that have been properly consumed
    pub(super) consumed: Vec<TrackedResource>,
    /// Next resource ID to assign
    pub(super) next_id: usize,
}

impl Default for StackState {
    fn default() -> Self {
        Self::new()
    }
}

impl StackState {
    pub fn new() -> Self {
        StackState {
            stack: Vec::new(),
            aux_stack: Vec::new(),
            consumed: Vec::new(),
            next_id: 0,
        }
    }

    /// Push an unknown value onto the stack
    pub fn push_unknown(&mut self) {
        self.stack.push(StackValue::Unknown);
    }

    /// Push a new tracked resource onto the stack
    pub fn push_resource(&mut self, kind: ResourceKind, line: usize, word: &str) {
        let resource = TrackedResource {
            kind,
            id: self.next_id,
            created_line: line,
            created_by: word.to_string(),
        };
        self.next_id += 1;
        self.stack.push(StackValue::Resource(resource));
    }

    /// Pop a value from the stack
    pub fn pop(&mut self) -> Option<StackValue> {
        self.stack.pop()
    }

    /// Peek at the top value without removing it
    pub fn peek(&self) -> Option<&StackValue> {
        self.stack.last()
    }

    /// Get stack depth
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Mark a resource as consumed (properly cleaned up)
    pub fn consume_resource(&mut self, resource: TrackedResource) {
        self.consumed.push(resource);
    }

    /// Get all resources still on the stack (potential leaks)
    pub fn remaining_resources(&self) -> Vec<&TrackedResource> {
        self.stack
            .iter()
            .filter_map(|v| match v {
                StackValue::Resource(r) => Some(r),
                StackValue::Unknown => None,
            })
            .collect()
    }

    /// Merge two stack states (for branch unification)
    /// Returns resources that are leaked in one branch but not the other
    pub fn merge(&self, other: &StackState) -> BranchMergeResult {
        let self_resources: HashMap<usize, &TrackedResource> = self
            .stack
            .iter()
            .filter_map(|v| match v {
                StackValue::Resource(r) => Some((r.id, r)),
                StackValue::Unknown => None,
            })
            .collect();

        let other_resources: HashMap<usize, &TrackedResource> = other
            .stack
            .iter()
            .filter_map(|v| match v {
                StackValue::Resource(r) => Some((r.id, r)),
                StackValue::Unknown => None,
            })
            .collect();

        let self_consumed: std::collections::HashSet<usize> =
            self.consumed.iter().map(|r| r.id).collect();
        let other_consumed: std::collections::HashSet<usize> =
            other.consumed.iter().map(|r| r.id).collect();

        let mut inconsistent = Vec::new();

        // Find resources consumed in one branch but not the other
        for (id, resource) in &self_resources {
            if other_consumed.contains(id) && !self_consumed.contains(id) {
                // Consumed in 'other' branch, still on stack in 'self'
                inconsistent.push(InconsistentResource {
                    resource: (*resource).clone(),
                    consumed_in_else: true,
                });
            }
        }

        for (id, resource) in &other_resources {
            if self_consumed.contains(id) && !other_consumed.contains(id) {
                // Consumed in 'self' branch, still on stack in 'other'
                inconsistent.push(InconsistentResource {
                    resource: (*resource).clone(),
                    consumed_in_else: false,
                });
            }
        }

        BranchMergeResult { inconsistent }
    }

    /// Compute a lattice join of two stack states for continuation after branches.
    ///
    /// The join is conservative:
    /// - Resources present in EITHER branch are tracked (we don't know which path was taken)
    /// - Resources are only marked consumed if consumed in BOTH branches
    /// - The next_id is taken from the max of both states
    ///
    /// This ensures we don't miss potential leaks from either branch.
    pub fn join(&self, other: &StackState) -> StackState {
        // Collect resource IDs consumed in each branch
        let other_consumed: std::collections::HashSet<usize> =
            other.consumed.iter().map(|r| r.id).collect();

        // Resources consumed in BOTH branches are definitely consumed
        let definitely_consumed: Vec<TrackedResource> = self
            .consumed
            .iter()
            .filter(|r| other_consumed.contains(&r.id))
            .cloned()
            .collect();

        // For the stack, we need to be careful. After if/else, stacks should
        // have the same depth (Seq requires balanced stack effects in branches).
        // We take the union of resources - if a resource appears in either
        // branch's stack, it should be tracked.
        //
        // Since we can't know which branch was taken, we use the then-branch
        // stack structure but ensure any resource from either branch is present.
        let mut joined_stack = self.stack.clone();

        // Collect resources from other branch that might not be in self
        let other_resources: HashMap<usize, TrackedResource> = other
            .stack
            .iter()
            .filter_map(|v| match v {
                StackValue::Resource(r) => Some((r.id, r.clone())),
                StackValue::Unknown => None,
            })
            .collect();

        // For each position, if other has a resource that self doesn't, use other's
        for (i, val) in joined_stack.iter_mut().enumerate() {
            if matches!(val, StackValue::Unknown)
                && i < other.stack.len()
                && let StackValue::Resource(r) = &other.stack[i]
            {
                *val = StackValue::Resource(r.clone());
            }
        }

        // Also check if other branch has resources we should track
        // (in case stacks have different structures due to analysis imprecision)
        let self_resource_ids: std::collections::HashSet<usize> = joined_stack
            .iter()
            .filter_map(|v| match v {
                StackValue::Resource(r) => Some(r.id),
                StackValue::Unknown => None,
            })
            .collect();

        for (id, resource) in other_resources {
            if !self_resource_ids.contains(&id) && !definitely_consumed.iter().any(|r| r.id == id) {
                // Resource from other branch not in our stack - add it
                // This handles cases where branches have different stack shapes
                joined_stack.push(StackValue::Resource(resource));
            }
        }

        // Join aux stacks conservatively (take the longer one to avoid false negatives)
        let joined_aux = if self.aux_stack.len() >= other.aux_stack.len() {
            self.aux_stack.clone()
        } else {
            other.aux_stack.clone()
        };

        StackState {
            stack: joined_stack,
            aux_stack: joined_aux,
            consumed: definitely_consumed,
            next_id: self.next_id.max(other.next_id),
        }
    }
}

/// Result of merging two branch states
#[derive(Debug)]
pub(crate) struct BranchMergeResult {
    /// Resources handled inconsistently between branches
    pub inconsistent: Vec<InconsistentResource>,
}

/// A resource handled differently in different branches
#[derive(Debug)]
pub(crate) struct InconsistentResource {
    pub resource: TrackedResource,
    /// True if consumed in else branch but not then branch
    pub consumed_in_else: bool,
}

// ============================================================================
// Cross-Word Analysis (Phase 2b)
// ============================================================================

/// Information about a word's resource behavior
#[derive(Debug, Clone, Default)]
pub(crate) struct WordResourceInfo {
    /// Resource kinds this word returns (resources on stack at word end)
    pub returns: Vec<ResourceKind>,
}
