// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Global allocator wrapper that counts allocations and bytes requested.
//!
//! Allocation counts are deterministic for a given input and independent
//! of machine load, so the perf ratchet locks them exactly (within a small
//! tolerance for hash-seed-dependent container growth) rather than locking
//! wall time, which drifts with hardware and background load. Graphviz's
//! C allocations go through libc and are not counted.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CountingAlloc;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

/// A point-in-time reading of the counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub allocs: usize,
    pub bytes: usize,
}

impl Snapshot {
    pub fn now() -> Self {
        Self {
            allocs: ALLOCS.load(Ordering::Relaxed),
            bytes: BYTES.load(Ordering::Relaxed),
        }
    }

    pub fn since(self, earlier: Snapshot) -> Snapshot {
        Snapshot {
            allocs: self.allocs - earlier.allocs,
            bytes: self.bytes - earlier.bytes,
        }
    }
}

// SAFETY: delegates every operation to `System` unchanged; the counters
// are plain relaxed atomics touched only on the allocation path.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new_size, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
