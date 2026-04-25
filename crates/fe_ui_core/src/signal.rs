// crates/fe_ui_core/src/signal.rs
//
// Fine-grained reactivity for Ferrum fe_ui.
//
// ─── Architecture ────────────────────────────────────────────────────────────
//
//  Signal<T>        — immediate reactive value. Writes update a version counter
//                     and notify the SignalGraph. GPU uniforms read this.
//
//  LayoutSignal<T>  — layout-affecting reactive value. Writes mark the owning
//                     entity dirty in the SignalGraph. Does NOT trigger an
//                     immediate Taffy recompute. Drains at the Sync Point.
//
//  Derived<T>       — computed value. Lazy — only recomputes when a dependency
//                     has a newer version than the last time it was read.
//                     Topo-sorted at the Sync Point so each node computes
//                     at most once per frame regardless of how many of its
//                     deps changed.
//
//  Scope            — a handle into the SignalGraph tied to one component's
//                     lifetime. When the component despawns, all signals
//                     created through that Scope are automatically dropped.
//
// ─── Sync Point contract ─────────────────────────────────────────────────────
//
//  Signals fire freely during the frame. At the Sync Point:
//    1. Queued physics writes (.queue()) drain → become normal .set() calls.
//    2. Dirty derived signals recompute (topo-sorted, once each).
//    3. Dirty layout nodes are collected and handed to fe_ui_taffy.
//
//  A layout signal written 1000 times in one frame causes exactly
//  ONE Taffy recompute — the final value wins.
//
// ─── Borrow safety ───────────────────────────────────────────────────────────
//
//  The SignalGraph is Rc<RefCell<SignalGraph>>. To prevent runtime panics:
//
//  1. The write queue is split into two lanes:
//       plain_queue  — (SignalId, FnOnce) for Signal::queue().
//                      Closure only touches the signal's Rc<RefCell<Inner>>.
//                      Never borrows the graph.
//       layout_queue — (SignalId, EntityId, FnOnce) for LayoutSignal::queue().
//                      Closure only touches the signal's Rc<RefCell<Inner>>.
//                      EntityId is stored separately so the drain loop can
//                      call mark_layout_dirty AFTER the closure runs.
//
//  2. drain_queues() is three explicit phases:
//       Phase A — move all entries into local Vecs (releases &mut self).
//       Phase B — execute each closure (no graph borrow alive).
//       Phase C — re-borrow graph to mark stale / dirty (closures are done).
//
//  This guarantees no closure ever runs while a borrow on the graph is live.
//
// ─── Thread safety ───────────────────────────────────────────────────────────
//
//  Ferrum's main loop is single-threaded. Rc<RefCell<T>> is deliberate.
//  Do not reach for Arc<Mutex<T>> here.
//
// ─────────────────────────────────────────────────────────────────────────────

#![allow(dead_code)]

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    rc::Rc,
};

// ─── Signal ID ───────────────────────────────────────────────────────────────

/// Unique identifier for every signal in the graph.
/// Cheap to copy. Used as graph node keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SignalId(u64);

impl SignalId {
    fn next() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        SignalId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for SignalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signal({})", self.0)
    }
}

// ─── Version counter ─────────────────────────────────────────────────────────

/// Monotonically increasing write counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Version(u64);

impl Version {
    fn increment(&mut self) -> Version {
        self.0 += 1;
        *self
    }
}

// ─── Inner storage ───────────────────────────────────────────────────────────

struct SignalInner<T> {
    id:      SignalId,
    value:   T,
    version: Version,
}

impl<T: fmt::Debug> fmt::Debug for SignalInner<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignalInner")
            .field("id",      &self.id)
            .field("value",   &self.value)
            .field("version", &self.version)
            .finish()
    }
}

// ─── Signal<T> ───────────────────────────────────────────────────────────────

/// An immediate reactive value.
///
/// Writes update a version counter and notify the SignalGraph synchronously.
/// Use for: colour, opacity, text content, shader parameters, visibility.
/// Do NOT use for layout properties — use LayoutSignal<T>.
#[derive(Clone)]
pub struct Signal<T: Clone + 'static> {
    inner: Rc<RefCell<SignalInner<T>>>,
    graph: Rc<RefCell<SignalGraph>>,
}

impl<T: Clone + 'static> Signal<T> {
    fn new(value: T, graph: Rc<RefCell<SignalGraph>>) -> Self {
        let id = SignalId::next();
        let inner = Rc::new(RefCell::new(SignalInner {
            id,
            value,
            version: Version::default(),
        }));
        graph.borrow_mut().register_signal(id);
        Self { inner, graph }
    }

    pub fn id(&self) -> SignalId { self.inner.borrow().id }
    pub fn version(&self) -> Version { self.inner.borrow().version }
    pub fn get(&self) -> T { self.inner.borrow().value.clone() }

    /// Write a new value immediately.
    pub fn set(&self, value: T) {
        let id = {
            let mut inner = self.inner.borrow_mut();
            inner.value = value;
            inner.version.increment();
            inner.id
        };
        // inner borrow is released before we touch the graph
        self.graph.borrow_mut().mark_signal_updated(id);
    }

    pub fn update(&self, f: impl FnOnce(T) -> T) {
        let new_value = f(self.inner.borrow().value.clone());
        self.set(new_value);
    }

    /// Queue a write for the next frame's Sync Point.
    ///
    /// Required inside physics event handlers.
    /// The closure captures only the signal's inner Rc — never the graph.
    /// The graph is updated in Phase C of drain_queues(), after the closure runs.
    pub fn queue(&self, value: T)
    where
        T: 'static,
    {
        let id          = self.inner.borrow().id;
        let inner_clone = Rc::clone(&self.inner);

        // Closure touches ONLY inner — no graph borrow.
        let closure: Box<dyn FnOnce()> = Box::new(move || {
            let mut inner = inner_clone.borrow_mut();
            inner.value   = value;
            inner.version.increment();
        });

        self.graph.borrow_mut().plain_queue.push_back((id, closure));
    }
}

impl<T: Clone + fmt::Debug + 'static> fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signal")
            .field("id",    &self.inner.borrow().id)
            .field("value", &self.inner.borrow().value)
            .finish()
    }
}

// ─── LayoutSignal<T> ─────────────────────────────────────────────────────────

/// A layout-affecting reactive value. Batched at the Sync Point.
///
/// Use for: width, height, flex-grow, padding, margin, display, position.
/// Writes mark the owning entity dirty — Taffy recomputes once per frame.
#[derive(Clone)]
pub struct LayoutSignal<T: Clone + 'static> {
    inner:     Rc<RefCell<SignalInner<T>>>,
    graph:     Rc<RefCell<SignalGraph>>,
    entity_id: crate::entity::EntityId,
}

impl<T: Clone + 'static> LayoutSignal<T> {
    fn new(
        value:     T,
        graph:     Rc<RefCell<SignalGraph>>,
        entity_id: crate::entity::EntityId,
    ) -> Self {
        let id = SignalId::next();
        let inner = Rc::new(RefCell::new(SignalInner {
            id,
            value,
            version: Version::default(),
        }));
        graph.borrow_mut().register_layout_signal(id, entity_id);
        Self { inner, graph, entity_id }
    }

    pub fn id(&self) -> SignalId { self.inner.borrow().id }
    pub fn get(&self) -> T { self.inner.borrow().value.clone() }

    /// Write a new value. Batched — drains at the Sync Point.
    pub fn set(&self, value: T) {
        let id = {
            let mut inner = self.inner.borrow_mut();
            inner.value = value;
            inner.version.increment();
            inner.id
        };
        // inner borrow released before touching graph
        self.graph.borrow_mut().mark_layout_dirty(id, self.entity_id);
    }

    pub fn update(&self, f: impl FnOnce(T) -> T) {
        let new_value = f(self.inner.borrow().value.clone());
        self.set(new_value);
    }

    /// Queue a deferred write. The closure captures only the signal's inner Rc.
    /// EntityId is stored separately for Phase C of drain_queues().
    pub fn queue(&self, value: T)
    where
        T: 'static,
    {
        let id          = self.inner.borrow().id;
        let entity_id   = self.entity_id;
        let inner_clone = Rc::clone(&self.inner);

        // Closure touches ONLY inner — no graph borrow.
        let closure: Box<dyn FnOnce()> = Box::new(move || {
            let mut inner = inner_clone.borrow_mut();
            inner.value   = value;
            inner.version.increment();
        });

        self.graph
            .borrow_mut()
            .layout_queue
            .push_back((id, entity_id, closure));
    }
}

impl<T: Clone + fmt::Debug + 'static> fmt::Debug for LayoutSignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayoutSignal")
            .field("id",        &self.inner.borrow().id)
            .field("value",     &self.inner.borrow().value)
            .field("entity_id", &self.entity_id)
            .finish()
    }
}

// ─── Derived<T> ──────────────────────────────────────────────────────────────

/// A computed value derived from one or more signals.
///
/// Lazy — only recomputes when a dependency has changed.
/// Topo-sorted at the Sync Point — each node computes at most once per frame.
#[derive(Clone)]
pub struct Derived<T: Clone + 'static> {
    id:      SignalId,
    compute: Rc<dyn Fn() -> T>,
    cached:  Rc<RefCell<Option<T>>>,
    graph:   Rc<RefCell<SignalGraph>>,
}

impl<T: Clone + 'static> Derived<T> {
    fn new(compute: impl Fn() -> T + 'static, graph: Rc<RefCell<SignalGraph>>) -> Self {
        let id      = SignalId::next();
        let compute = Rc::new(compute);
        let cached  = Rc::new(RefCell::new(None::<T>));
        graph.borrow_mut().register_derived(id);
        Self { id, compute, cached, graph }
    }

    pub fn id(&self) -> SignalId { self.id }

    pub fn get(&self) -> T {
        let is_stale = self.graph.borrow().is_derived_stale(self.id);

        if is_stale || self.cached.borrow().is_none() {
            let value = (self.compute)();
            *self.cached.borrow_mut() = Some(value.clone());
            self.graph.borrow_mut().clear_derived_stale(self.id);
            value
        } else {
            self.cached.borrow().clone().unwrap()
        }
    }
}

impl<T: Clone + fmt::Debug + 'static> fmt::Debug for Derived<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Derived")
            .field("id",     &self.id)
            .field("cached", &self.cached.borrow())
            .finish()
    }
}

// ─── SignalGraph ──────────────────────────────────────────────────────────────

/// The reactive dependency graph for the entire application.
///
/// # Write queue borrow safety — two lane design
///
///   plain_queue  — Signal::queue() entries: (SignalId, FnOnce).
///                  Closure only touches the signal's inner value.
///
///   layout_queue — LayoutSignal::queue() entries: (SignalId, EntityId, FnOnce).
///                  Closure only touches the signal's inner value.
///                  EntityId carried so drain_queues() marks dirty in Phase C.
///
/// drain_queues() is always three phases:
///   A) drain VecDeques into local Vecs → releases &mut self
///   B) execute closures              → no graph borrow alive
///   C) mark_signal_updated / mark_layout_dirty → safe re-borrow
pub struct SignalGraph {
    nodes:                  HashSet<SignalId>,
    dependencies:           HashMap<SignalId, HashSet<SignalId>>,
    subscribers:            HashMap<SignalId, HashSet<SignalId>>,
    stale_derived:          HashSet<SignalId>,
    dirty_layout:           HashMap<SignalId, crate::entity::EntityId>,
    layout_signal_entities: HashMap<SignalId, crate::entity::EntityId>,

    /// Signal::queue() entries. Closure touches inner value only.
    plain_queue:  VecDeque<(SignalId, Box<dyn FnOnce()>)>,

    /// LayoutSignal::queue() entries. Closure touches inner value only.
    layout_queue: VecDeque<(SignalId, crate::entity::EntityId, Box<dyn FnOnce()>)>,
}

impl SignalGraph {
    pub fn new() -> Self {
        Self {
            nodes:                  HashSet::new(),
            dependencies:           HashMap::new(),
            subscribers:            HashMap::new(),
            stale_derived:          HashSet::new(),
            dirty_layout:           HashMap::new(),
            layout_signal_entities: HashMap::new(),
            plain_queue:            VecDeque::new(),
            layout_queue:           VecDeque::new(),
        }
    }

    // ── Registration ─────────────────────────────────────────────────────────

    pub(crate) fn register_signal(&mut self, id: SignalId) {
        self.nodes.insert(id);
    }

    pub(crate) fn register_layout_signal(
        &mut self,
        id:        SignalId,
        entity_id: crate::entity::EntityId,
    ) {
        self.nodes.insert(id);
        self.layout_signal_entities.insert(id, entity_id);
    }

    pub(crate) fn register_derived(&mut self, id: SignalId) {
        self.nodes.insert(id);
        self.dependencies.entry(id).or_default();
    }

    // ── Dirty / stale marking ─────────────────────────────────────────────────

    /// BFS propagation of stale flags to all downstream derived signals.
    pub(crate) fn mark_signal_updated(&mut self, id: SignalId) {
        let mut to_visit = vec![id];
        let mut visited  = HashSet::new();

        while let Some(current) = to_visit.pop() {
            if !visited.insert(current) { continue; }
            if let Some(subs) = self.subscribers.get(&current).cloned() {
                for sub in subs {
                    self.stale_derived.insert(sub);
                    to_visit.push(sub);
                }
            }
        }
    }

    pub(crate) fn mark_layout_dirty(
        &mut self,
        id:        SignalId,
        entity_id: crate::entity::EntityId,
    ) {
        self.dirty_layout.insert(id, entity_id);
        self.mark_signal_updated(id);
    }

    pub(crate) fn is_derived_stale(&self, id: SignalId) -> bool {
        self.stale_derived.contains(&id)
    }

    pub(crate) fn clear_derived_stale(&mut self, id: SignalId) {
        self.stale_derived.remove(&id);
    }

    // ── Sync Point ───────────────────────────────────────────────────────────

    /// Drain all queued .queue() writes from the previous frame.
    ///
    /// Three-phase protocol — no closure ever runs while the graph is borrowed.
    ///
    ///   Phase A: move all queue entries into local Vecs (releases &mut self).
    ///   Phase B: execute closures (no graph borrow — touches inner values only).
    ///   Phase C: mark_signal_updated / mark_layout_dirty (safe re-borrow).
    pub fn drain_queues(&mut self) {
        // ── Phase A ──────────────────────────────────────────────────────────
        // Drain both queues into owned local Vecs.
        // After this, self.plain_queue and self.layout_queue are empty.
        // The &mut self borrow ends at the closing brace of this block... wait,
        // we're still in the same &mut self method. The key insight is that
        // after drain().collect(), the VecDeques are empty and we hold owned
        // Vecs. The closures are now owned by us, not by self.
        // We then call self.mark_* in Phase C — that re-borrows self, which
        // is fine because the closures (which held no graph reference) are
        // already consumed by then.

        let plain_entries: Vec<(SignalId, Box<dyn FnOnce()>)> =
            self.plain_queue.drain(..).collect();

        let layout_entries: Vec<(SignalId, crate::entity::EntityId, Box<dyn FnOnce()>)> =
            self.layout_queue.drain(..).collect();

        // ── Phase B ──────────────────────────────────────────────────────────
        // Execute closures. Each closure ONLY borrows its signal's inner
        // Rc<RefCell<SignalInner<T>>>. It does NOT touch self (the graph).
        // Therefore self is not borrowed during closure execution.

        let plain_ids: Vec<SignalId> = plain_entries
            .into_iter()
            .map(|(id, closure)| {
                closure(); // safe — no graph borrow
                id
            })
            .collect();

        let layout_results: Vec<(SignalId, crate::entity::EntityId)> = layout_entries
            .into_iter()
            .map(|(id, entity_id, closure)| {
                closure(); // safe — no graph borrow
                (id, entity_id)
            })
            .collect();

        // ── Phase C ──────────────────────────────────────────────────────────
        // All closures are done. We now re-borrow self to update graph state.
        // No closures are alive. No Rc<RefCell<SignalGraph>> borrows are active.
        // This is the only place where we borrow self after the closures ran.

        for id in plain_ids {
            self.mark_signal_updated(id);
        }

        for (id, entity_id) in layout_results {
            self.mark_layout_dirty(id, entity_id);
        }
    }

    /// Collect all dirty layout entity IDs. Clears the set.
    /// Called at Step 3 of the Sync Point.
    pub fn take_layout_dirty(&mut self) -> Vec<crate::entity::EntityId> {
        let mut seen = HashSet::new();
        let mut entities: Vec<crate::entity::EntityId> = self
            .dirty_layout
            .values()
            .copied()
            .filter(|e| seen.insert(*e))
            .collect();

        entities.sort_unstable();
        self.dirty_layout.clear();
        entities
    }

    /// Topologically sorted order of stale derived signals.
    /// Kahn's algorithm. Deterministic — each batch sorted before processing.
    pub fn topo_sorted_derived(&self) -> Vec<SignalId> {
        let mut in_degree: HashMap<SignalId, usize> = HashMap::new();
        let mut adj: HashMap<SignalId, Vec<SignalId>> = HashMap::new();

        for &derived in &self.stale_derived {
            in_degree.entry(derived).or_insert(0);
            if let Some(deps) = self.dependencies.get(&derived) {
                for &dep in deps {
                    if self.stale_derived.contains(&dep) {
                        adj.entry(dep).or_default().push(derived);
                        *in_degree.entry(derived).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut queue: Vec<SignalId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort_unstable();

        let mut sorted = Vec::with_capacity(self.stale_derived.len());

        while !queue.is_empty() {
            queue.sort_unstable();
            let id = queue.remove(0);
            sorted.push(id);

            if let Some(neighbors) = adj.get(&id) {
                for &neighbor in neighbors {
                    let deg = in_degree.entry(neighbor).or_insert(0);
                    if *deg > 0 { *deg -= 1; }
                    if *deg == 0 { queue.push(neighbor); }
                }
            }
        }

        sorted
    }

    /// Clear all stale flags after Sync Point recomputation is complete.
    pub fn clear_stale(&mut self) {
        self.stale_derived.clear();
    }

    // ── Diagnostics ──────────────────────────────────────────────────────────

    pub fn signal_count(&self) -> usize { self.nodes.len() }

    pub fn dirty_layout_count(&self) -> usize { self.dirty_layout.len() }

    pub fn queued_write_count(&self) -> usize {
        self.plain_queue.len() + self.layout_queue.len()
    }
}

impl Default for SignalGraph {
    fn default() -> Self { Self::new() }
}

impl fmt::Debug for SignalGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignalGraph")
            .field("signals",       &self.nodes.len())
            .field("dirty_layout",  &self.dirty_layout.len())
            .field("stale_derived", &self.stale_derived.len())
            .field("plain_queue",   &self.plain_queue.len())
            .field("layout_queue",  &self.layout_queue.len())
            .finish()
    }
}

// ─── Scope ───────────────────────────────────────────────────────────────────

/// A handle into the SignalGraph tied to one component's lifetime.
/// Passed as `cx` to every component function.
#[derive(Clone)]
pub struct Scope {
    pub(crate) entity_id:     crate::entity::EntityId,
    pub(crate) graph:         Rc<RefCell<SignalGraph>>,
    pub(crate) owned_signals: Rc<RefCell<Vec<SignalId>>>,
}

impl Scope {
    pub fn new(
        entity_id: crate::entity::EntityId,
        graph:     Rc<RefCell<SignalGraph>>,
    ) -> Self {
        Self {
            entity_id,
            graph,
            owned_signals: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn entity_id(&self) -> crate::entity::EntityId { self.entity_id }
}

impl fmt::Debug for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scope")
            .field("entity_id",     &self.entity_id)
            .field("owned_signals", &self.owned_signals.borrow().len())
            .finish()
    }
}

// ─── Constructor functions ────────────────────────────────────────────────────

/// Create an immediate reactive signal.
/// Use for: colour, opacity, text, shader parameters.
pub fn use_signal<T: Clone + 'static>(cx: &Scope, init: impl FnOnce() -> T) -> Signal<T> {
    let signal = Signal::new(init(), Rc::clone(&cx.graph));
    cx.owned_signals.borrow_mut().push(signal.id());
    signal
}

/// Create a layout-affecting reactive signal. Batched at the Sync Point.
/// Use for: width, height, flex, padding, margin, position.
pub fn use_layout_signal<T: Clone + 'static>(
    cx:   &Scope,
    init: impl FnOnce() -> T,
) -> LayoutSignal<T> {
    let signal = LayoutSignal::new(init(), Rc::clone(&cx.graph), cx.entity_id);
    cx.owned_signals.borrow_mut().push(signal.id());
    signal
}

/// Create a derived (computed) signal. Lazy, topo-sorted.
pub fn use_derived<T: Clone + 'static>(
    cx:      &Scope,
    compute: impl Fn() -> T + 'static,
) -> Derived<T> {
    let derived = Derived::new(compute, Rc::clone(&cx.graph));
    cx.owned_signals.borrow_mut().push(derived.id());
    derived
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;
    use crate::entity::EntityId;

    fn make_scope() -> Scope {
        let graph = Rc::new(RefCell::new(SignalGraph::new()));
        let mut store: SlotMap<EntityId, ()> = SlotMap::with_key();
        let entity_id = store.insert(());
        Scope::new(entity_id, graph)
    }

    // ── Signal<T> ────────────────────────────────────────────────────────────

    #[test]
    fn signal_initial_value() {
        let cx  = make_scope();
        let sig = use_signal(&cx, || 42_i32);
        assert_eq!(sig.get(), 42);
    }

    #[test]
    fn signal_set_updates_value() {
        let cx  = make_scope();
        let sig = use_signal(&cx, || 0_i32);
        sig.set(99);
        assert_eq!(sig.get(), 99);
    }

    #[test]
    fn signal_set_increments_version() {
        let cx  = make_scope();
        let sig = use_signal(&cx, || 0_i32);
        let v0  = sig.version();
        sig.set(1);
        let v1 = sig.version();
        sig.set(2);
        let v2 = sig.version();
        assert!(v1 > v0);
        assert!(v2 > v1);
    }

    #[test]
    fn signal_update_closure() {
        let cx  = make_scope();
        let sig = use_signal(&cx, || 10_i32);
        sig.update(|v| v * 2);
        assert_eq!(sig.get(), 20);
    }

    #[test]
    fn signal_multiple_writes_same_frame() {
        let cx  = make_scope();
        let sig = use_signal(&cx, || 0_i32);
        sig.set(1);
        sig.set(2);
        sig.set(3);
        assert_eq!(sig.get(), 3);
    }

    #[test]
    fn signal_queue_drains_at_sync_point() {
        let cx  = make_scope();
        let sig = use_signal(&cx, || 0_i32);

        sig.queue(42);

        assert_eq!(sig.get(), 0);
        assert_eq!(cx.graph.borrow().queued_write_count(), 1);

        cx.graph.borrow_mut().drain_queues();

        assert_eq!(sig.get(), 42);
        assert_eq!(cx.graph.borrow().queued_write_count(), 0);
    }

    #[test]
    fn signal_queue_multiple_last_wins() {
        let cx  = make_scope();
        let sig = use_signal(&cx, || 0_i32);
        sig.queue(1);
        sig.queue(2);
        sig.queue(3);
        assert_eq!(cx.graph.borrow().queued_write_count(), 3);
        cx.graph.borrow_mut().drain_queues();
        assert_eq!(sig.get(), 3);
        assert_eq!(cx.graph.borrow().queued_write_count(), 0);
    }

    // ── LayoutSignal<T> ──────────────────────────────────────────────────────

    #[test]
    fn layout_signal_initial_value() {
        let cx  = make_scope();
        let sig = use_layout_signal(&cx, || 300.0_f32);
        assert_eq!(sig.get(), 300.0);
    }

    #[test]
    fn layout_signal_set_marks_entity_dirty() {
        let cx  = make_scope();
        let sig = use_layout_signal(&cx, || 300.0_f32);
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 0);
        sig.set(500.0);
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 1);
        assert_eq!(sig.get(), 500.0);
    }

    #[test]
    fn layout_signal_multiple_writes_one_dirty_entry() {
        let cx  = make_scope();
        let sig = use_layout_signal(&cx, || 0.0_f32);
        for i in 0..1000 { sig.set(i as f32); }
        // 1000 writes → 1 dirty entry (same entity deduped by HashMap key)
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 1);
        assert_eq!(sig.get(), 999.0);
    }

    #[test]
    fn layout_signal_take_dirty_clears_set() {
        let cx  = make_scope();
        let sig = use_layout_signal(&cx, || 0.0_f32);
        sig.set(1.0);
        let dirty = cx.graph.borrow_mut().take_layout_dirty();
        assert_eq!(dirty.len(), 1);
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 0);
    }

    #[test]
    fn layout_signal_queue_deferred() {
        let cx  = make_scope();
        let sig = use_layout_signal(&cx, || 0.0_f32);

        sig.queue(99.0);

        // Before drain — value unchanged, not yet dirty
        assert_eq!(sig.get(), 0.0);
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 0);
        assert_eq!(cx.graph.borrow().queued_write_count(), 1);

        // Sync Point drain — three phases, no double borrow
        cx.graph.borrow_mut().drain_queues();

        // After drain — value updated AND entity marked dirty
        assert_eq!(sig.get(), 99.0);
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 1);
        assert_eq!(cx.graph.borrow().queued_write_count(), 0);
    }

    #[test]
    fn layout_signal_queue_entity_in_dirty_set() {
        let cx        = make_scope();
        let entity_id = cx.entity_id();
        let sig       = use_layout_signal(&cx, || 0.0_f32);

        sig.queue(50.0);
        cx.graph.borrow_mut().drain_queues();

        let dirty = cx.graph.borrow_mut().take_layout_dirty();
        assert!(dirty.contains(&entity_id));
    }

    // ── SignalGraph ───────────────────────────────────────────────────────────

    #[test]
    fn graph_signal_count_tracks_registration() {
        let cx = make_scope();
        assert_eq!(cx.graph.borrow().signal_count(), 0);
        let _a = use_signal(&cx, || 0_i32);
        assert_eq!(cx.graph.borrow().signal_count(), 1);
        let _b = use_signal(&cx, || 0_i32);
        assert_eq!(cx.graph.borrow().signal_count(), 2);
        let _c = use_layout_signal(&cx, || 0.0_f32);
        assert_eq!(cx.graph.borrow().signal_count(), 3);
    }

    #[test]
    fn graph_topo_sort_empty_when_no_stale() {
        let cx = make_scope();
        assert!(cx.graph.borrow().topo_sorted_derived().is_empty());
    }

    #[test]
    fn graph_mark_signal_updated_marks_subscribers_stale() {
        let cx    = make_scope();
        let graph = Rc::clone(&cx.graph);

        let source_id  = SignalId::next();
        let derived_id = SignalId::next();

        {
            let mut g = graph.borrow_mut();
            g.register_signal(source_id);
            g.register_derived(derived_id);
            g.dependencies.entry(derived_id).or_default().insert(source_id);
            g.subscribers.entry(source_id).or_default().insert(derived_id);
        }

        assert!(!graph.borrow().is_derived_stale(derived_id));
        graph.borrow_mut().mark_signal_updated(source_id);
        assert!(graph.borrow().is_derived_stale(derived_id));
    }

    #[test]
    fn graph_clear_stale_resets_all_flags() {
        let cx    = make_scope();
        let graph = Rc::clone(&cx.graph);
        let src   = SignalId::next();
        let drv   = SignalId::next();

        {
            let mut g = graph.borrow_mut();
            g.register_signal(src);
            g.register_derived(drv);
            g.dependencies.entry(drv).or_default().insert(src);
            g.subscribers.entry(src).or_default().insert(drv);
            g.mark_signal_updated(src);
        }

        assert!(graph.borrow().is_derived_stale(drv));
        graph.borrow_mut().clear_stale();
        assert!(!graph.borrow().is_derived_stale(drv));
    }

    #[test]
    fn graph_topo_sort_linear_chain() {
        // dependency chain: a ← b ← c  (c depends on b, b depends on a)
        // correct topo order: a, b, c
        let cx    = make_scope();
        let graph = Rc::clone(&cx.graph);

        let a = SignalId::next();
        let b = SignalId::next();
        let c = SignalId::next();

        {
            let mut g = graph.borrow_mut();
            g.register_derived(a);
            g.register_derived(b);
            g.register_derived(c);
            // b depends on a
            g.dependencies.entry(b).or_default().insert(a);
            g.subscribers.entry(a).or_default().insert(b);
            // c depends on b
            g.dependencies.entry(c).or_default().insert(b);
            g.subscribers.entry(b).or_default().insert(c);
            // mark all stale
            g.stale_derived.insert(a);
            g.stale_derived.insert(b);
            g.stale_derived.insert(c);
        }

        let sorted = graph.borrow().topo_sorted_derived();
        let pos    = |id: SignalId| sorted.iter().position(|&x| x == id).unwrap();
        assert!(pos(a) < pos(b), "a must come before b");
        assert!(pos(b) < pos(c), "b must come before c");
    }

    // ── Scope ─────────────────────────────────────────────────────────────────

    #[test]
    fn scope_tracks_owned_signals() {
        let cx = make_scope();
        assert_eq!(cx.owned_signals.borrow().len(), 0);
        let _a = use_signal(&cx, || 0_i32);
        let _b = use_signal(&cx, || 0.0_f32);
        let _c = use_layout_signal(&cx, || false);
        assert_eq!(cx.owned_signals.borrow().len(), 3);
    }

    #[test]
    fn scope_entity_id_matches_dirty_entry() {
        let cx        = make_scope();
        let entity_id = cx.entity_id();
        let sig       = use_layout_signal(&cx, || 0.0_f32);
        sig.set(1.0);
        let dirty = cx.graph.borrow_mut().take_layout_dirty();
        assert!(dirty.contains(&entity_id));
    }

    // ── Mixed queue drain ─────────────────────────────────────────────────────

    #[test]
    fn drain_queues_handles_plain_and_layout_together() {
        let cx = make_scope();

        let plain_sig  = use_signal(&cx, || 0_i32);
        let layout_sig = use_layout_signal(&cx, || 0.0_f32);

        plain_sig.queue(7);
        layout_sig.queue(3.14);

        assert_eq!(cx.graph.borrow().queued_write_count(), 2);

        cx.graph.borrow_mut().drain_queues();

        assert_eq!(plain_sig.get(),  7);
        assert_eq!(layout_sig.get(), 3.14);
        assert_eq!(cx.graph.borrow().queued_write_count(), 0);
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 1);
    }
}
        
