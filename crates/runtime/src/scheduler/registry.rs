//! Lock-free strand registry (diagnostics feature only).
//!
//! A fixed-size array of slots for tracking active strands without locks.
//! Each slot stores a strand ID (0 = free) and spawn timestamp.
//!
//! Design principles:
//! - Fixed size: No dynamic allocation, predictable memory footprint
//! - Lock-free: All operations use atomic CAS, no mutex contention
//! - Bounded: If registry is full, strands still run but aren't tracked
//! - Zero cost when not querying: Only diagnostics reads the registry
//!
//! Slot encoding:
//! - `strand_id == 0`: slot is free
//! - `strand_id > 0`: slot contains an active strand
//!
//! The registry size can be configured via `SEQ_STRAND_REGISTRY_SIZE` env var.
//! Default is 1024 slots, which is sufficient for most applications.
//!
//! When the "diagnostics" feature is disabled, the registry is not compiled,
//! eliminating the `SystemTime::now()` syscall and O(n) scans on every spawn.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Default strand registry size (number of trackable concurrent strands)
const DEFAULT_REGISTRY_SIZE: usize = 1024;

/// A slot in the strand registry
///
/// Uses two atomics to store strand info without locks.
/// A slot is free when `strand_id == 0`.
pub struct StrandSlot {
    /// Strand ID (0 = free, >0 = active strand)
    pub strand_id: AtomicU64,
    /// Spawn timestamp (seconds since UNIX epoch, for detecting stuck strands)
    pub spawn_time: AtomicU64,
}

impl StrandSlot {
    const fn new() -> Self {
        Self {
            strand_id: AtomicU64::new(0),
            spawn_time: AtomicU64::new(0),
        }
    }
}

/// Lock-free strand registry
///
/// Provides O(n) registration (scan for free slot) and O(n) unregistration.
/// This is acceptable because:
/// 1. N is bounded (default 1024)
/// 2. Registration/unregistration are infrequent compared to strand work
/// 3. No locks means no contention, just atomic ops
pub struct StrandRegistry {
    slots: Box<[StrandSlot]>,
    /// Number of slots that couldn't be registered (registry full)
    pub overflow_count: AtomicU64,
}

impl StrandRegistry {
    /// Create a new registry with the given capacity
    pub(super) fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(StrandSlot::new());
        }
        Self {
            slots: slots.into_boxed_slice(),
            overflow_count: AtomicU64::new(0),
        }
    }

    /// Register a strand, returning the slot index if successful
    ///
    /// Uses CAS to atomically claim a free slot.
    /// Returns None if the registry is full (strand still runs, just not tracked).
    pub fn register(&self, strand_id: u64) -> Option<usize> {
        let spawn_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Scan for a free slot
        for (idx, slot) in self.slots.iter().enumerate() {
            // Set spawn time first, before claiming the slot
            // This prevents a race where a reader sees strand_id != 0 but spawn_time == 0
            // If we fail to claim the slot, the owner will overwrite this value anyway
            slot.spawn_time.store(spawn_time, Ordering::Relaxed);

            // Try to claim this slot (CAS from 0 to strand_id)
            // AcqRel ensures the spawn_time write above is visible before strand_id becomes non-zero
            if slot
                .strand_id
                .compare_exchange(0, strand_id, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return Some(idx);
            }
        }

        // Registry full - track overflow but strand still runs
        self.overflow_count.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Unregister a strand by ID
    ///
    /// Scans for the slot containing this strand ID and clears it.
    /// Returns true if found and cleared, false if not found.
    ///
    /// Note: ABA problem is not a concern here because strand IDs are monotonically
    /// increasing u64 values. ID reuse would require 2^64 spawns, which is practically
    /// impossible (at 1 billion spawns/sec, it would take ~584 years).
    pub fn unregister(&self, strand_id: u64) -> bool {
        for slot in self.slots.iter() {
            // Check if this slot contains our strand
            if slot
                .strand_id
                .compare_exchange(strand_id, 0, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                // Successfully cleared the slot
                slot.spawn_time.store(0, Ordering::Release);
                return true;
            }
        }
        false
    }

    /// Iterate over active strands (for diagnostics)
    ///
    /// Returns an iterator of (strand_id, spawn_time) for non-empty slots.
    /// Note: This is a snapshot and may be slightly inconsistent due to concurrent updates.
    pub fn active_strands(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.slots.iter().filter_map(|slot| {
            // Acquire on strand_id synchronizes with the Release in register()
            let id = slot.strand_id.load(Ordering::Acquire);
            if id > 0 {
                // Relaxed is sufficient here - we've already synchronized via strand_id Acquire
                // and spawn_time is written before strand_id in register()
                let time = slot.spawn_time.load(Ordering::Relaxed);
                Some((id, time))
            } else {
                None
            }
        })
    }

    /// Get the registry capacity
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
}

// Global strand registry (lazy initialized)
static STRAND_REGISTRY: OnceLock<StrandRegistry> = OnceLock::new();

/// Get or initialize the global strand registry
pub fn strand_registry() -> &'static StrandRegistry {
    STRAND_REGISTRY.get_or_init(|| {
        let size = std::env::var("SEQ_STRAND_REGISTRY_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_REGISTRY_SIZE);
        StrandRegistry::new(size)
    })
}
