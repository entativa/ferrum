//! signal.rs — Fine-grained reactivity for Ferrum.
//!
//! Two signal types implement the Sync Point contract:
//!
//! - `Signal<T>`        — immediate, GPU-only. Use for colour, opacity, text.
//! - `LayoutSignal<T>`  — batched, layout-affecting. Use for width, height, flex.
//!
//! See SIGNAL_GRAPH.md for the full data flow specification.

// TODO: implement Signal<T>, LayoutSignal<T>, SignalGraph, use_derived
// Key invariants:
//   - use_layout_signal writes mark a node dirty; they do NOT trigger
//     immediate Taffy recomputation.
//   - Taffy recomputes exactly once per frame at the Sync Point.
//   - Physics event handlers must use .queue() not .set() to avoid
//     feedback loops back into the current physics step.

/// Marker for a reactive value that updates a GPU uniform immediately.
pub struct Signal<T>(pub T);

/// Marker for a reactive value that drives Taffy layout.
/// Batched — drains at the Sync Point, not on write.
pub struct LayoutSignal<T>(pub T);

pub fn use_signal<T: Clone>(_cx: &(), init: impl FnOnce() -> T) -> Signal<T> {
    Signal(init())
}

pub fn use_layout_signal<T: Clone>(_cx: &(), init: impl FnOnce() -> T) -> LayoutSignal<T> {
    LayoutSignal(init())
}

pub fn use_derived<T: Clone>(_cx: &(), _compute: impl Fn() -> T) -> Signal<T> {
    todo!("use_derived — implement lazy topo-sorted derived signals")
}
