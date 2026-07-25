//! Shared counting global allocator for the Phase-3.1a action-cost benchmark
//! (Task 6). Included via `#[path]` by BOTH the worst-case bound test
//! (`tests/action_source_worstcase.rs`) and the recording example
//! (`examples/action_cost.rs`), so the meter has ONE definition and cannot drift.
//! Each includer is a separate binary, so each links its own private
//! `#[global_allocator]` — there is no conflict with the crate's own
//! `#[cfg(test)]` lib-only probe (`game/search_actions.rs`'s `alloc_probe`,
//! which exists only in the `--lib` build). It lives in a `tests/` SUBDIRECTORY
//! so cargo does not compile it as a standalone test target (the
//! `tests/common/` convention).
//!
//! METER DEFINITION (spec §4, PINNED). Two figures per measured region:
//!   - `allocs` — the count of allocation events (`alloc` + `alloc_zeroed` +
//!     `realloc`, matching the lib probe's convention; `dealloc` is not an
//!     allocation event). DETERMINISTIC across runs; this is the figure the
//!     worst-case test HARD-ASSERTS.
//!   - `peak_bytes` — **peak OUTSTANDING requested bytes**: the high-water mark
//!     of (Σ requested alloc sizes − Σ freed sizes) over the region. This is NOT
//!     RSS (unmeasurable dependency-free) and NOT cumulative; it is the live-set
//!     high-water mark. RECORDED only (machine-variable), never a hard gate.
//!
//! Correctness of the meter itself: every `GlobalAlloc` method forwards to the
//! process `System` allocator unchanged, so the contract is upheld; the only
//! added work is non-allocating thread-local counter arithmetic. Counting is
//! armed per-THREAD by [`measure`], so parallel `cargo test` threads never
//! perturb one another's figures. Const-initialized thread-locals never allocate
//! on access (no lazy init, no destructor), so the counters can be read from
//! inside `alloc` without re-entering it. `dealloc` receives the freed
//! `Layout`, so outstanding bytes are tracked WITHOUT a pointer→size map (which
//! would itself allocate); a free of memory allocated before the region was
//! armed simply drives the signed outstanding counter down and never raises the
//! peak, so the high-water mark stays correct.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    /// Whether THIS thread is inside a [`measure`] region (counting is on).
    static ARMED: Cell<bool> = const { Cell::new(false) };
    /// Allocation events counted since the region armed.
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    /// Net live requested bytes since the region armed (signed: a pre-region
    /// free can push it below zero, which never raises [`PEAK`]).
    static OUTSTANDING: Cell<isize> = const { Cell::new(0) };
    /// High-water mark of [`OUTSTANDING`] since the region armed.
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

struct Counting;

// SAFETY: every method forwards to the process `System` allocator with the same
// pointer/layout, so the `GlobalAlloc` contract is upheld unchanged; the only
// added work is the non-allocating thread-local accounting in `record`.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size() as isize, true);
        System.alloc(layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size() as isize, true);
        System.alloc_zeroed(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // A free is not an allocation EVENT, but it does return live bytes.
        record(-(layout.size() as isize), false);
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Frees `layout.size()` and requests `new_size`: one allocation event,
        // outstanding adjusted by the delta.
        record(new_size as isize - layout.size() as isize, true);
        System.realloc(ptr, layout, new_size)
    }
}

/// Apply one (de)allocation to the armed thread's counters: bump the event count
/// when `is_alloc_event`, adjust net outstanding bytes by `delta`, and raise the
/// peak. A no-op when the thread is not inside a [`measure`] region. `try_with`
/// tolerates access during thread teardown (returns without counting).
fn record(delta: isize, is_alloc_event: bool) {
    if !ARMED.try_with(Cell::get).unwrap_or(false) {
        return;
    }
    if is_alloc_event {
        let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
    }
    let now = match OUTSTANDING.try_with(|o| {
        let v = o.get() + delta;
        o.set(v);
        v
    }) {
        Ok(v) => v,
        Err(_) => return,
    };
    let _ = PEAK.try_with(|p| {
        if now > p.get() {
            p.set(now);
        }
    });
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// The recorded meter for one [`measure`] region.
#[derive(Debug, Clone, Copy)]
pub struct Meter {
    /// Number of allocation events (`alloc` + `alloc_zeroed` + `realloc`) — the
    /// deterministic, hard-asserted figure.
    pub allocs: usize,
    /// Peak outstanding requested bytes (high-water mark of net live bytes) — a
    /// recorded curve, machine-variable, never a hard gate.
    pub peak_bytes: usize,
}

/// Run `f` with the meter armed on THIS thread, returning its value and the
/// region's [`Meter`]. Counters reset before arming (the resets are themselves
/// uncounted and non-allocating), so PRIOR activity never leaks in. NOT
/// RE-ENTRANT: a nested `measure` would wipe the outer region's counters and
/// disarm its remainder — under-counting whose only failure mode on the `<
/// BOUND` gate is a false PASS — so nesting is asserted against at entry.
pub fn measure<R>(f: impl FnOnce() -> R) -> (R, Meter) {
    ARMED.with(|a| {
        assert!(!a.get(), "action_cost_meter::measure is not re-entrant (nested region would corrupt the outer count)");
    });
    ALLOCS.with(|n| n.set(0));
    OUTSTANDING.with(|o| o.set(0));
    PEAK.with(|p| p.set(0));
    ARMED.with(|a| a.set(true));
    let r = f();
    ARMED.with(|a| a.set(false));
    let allocs = ALLOCS.with(Cell::get);
    let peak_bytes = PEAK.with(Cell::get).max(0) as usize;
    (r, Meter { allocs, peak_bytes })
}
