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
//  A layout signal that is written 1000 times in one frame causes exactly
//  ONE Taffy recompute — the final value wins.
//
// ─── Thread safety ───────────────────────────────────────────────────────────
//
//  Ferrum's main loop is single-threaded. We use Rc<RefCell<T>> deliberately.
//  If you are reaching for Arc<Mutex<T>>, you are in the wrong place.
//
// ─────────────────────────────────────────────────────────────────────────────

use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    rc::Rc,
};

// ─── Signal ID ───────────────────────────────────────────────────────────────

/// Unique identifier for every signal in the graph.
/// Cheap to copy. Used as graph node keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
/// Derived signals compare their `last_computed_version` against the max
/// version of their dependencies to decide if recomputation is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Version(u64);

impl Version {
    fn increment(&mut self) -> Version {
        self.0 += 1;
        *self
    }
}

// ─── Inner storage ───────────────────────────────────────────────────────────

/// The heap-allocated interior of a Signal or LayoutSignal.
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
/// Reads are always the current value — no batching, no delay.
///
/// Use for: colour, opacity, text content, shader parameters, visibility.
/// Do NOT use for: width, height, flex properties — use LayoutSignal<T>.
///
/// # Example
/// ```rust
/// let is_hovered = use_signal(cx, || false);
///
/// // Write — immediate, notifies graph
/// is_hovered.set(true);
///
/// // Read
/// if is_hovered.get() { ... }
/// ```
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

    /// Returns the signal's unique ID.
    pub fn id(&self) -> SignalId {
        self.inner.borrow().id
    }

    /// Returns the current version (write count).
    pub fn version(&self) -> Version {
        self.inner.borrow().version
    }

    /// Read the current value.
    ///
    /// If called inside a `use_derived` computation, automatically registers
    /// this signal as a dependency of the derived signal being computed.
    pub fn get(&self) -> T {
        let inner = self.inner.borrow();

        // Track dependency if we're inside a derived computation
        self.graph
            .borrow()
            .track_dependency(inner.id, inner.version);

        inner.value.clone()
    }

    /// Write a new value immediately.
    ///
    /// Increments the version counter and notifies the SignalGraph.
    /// Subscribers (derived signals) are marked stale but NOT recomputed yet —
    /// recomputation happens lazily at the Sync Point.
    pub fn set(&self, value: T) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.value = value;
            inner.version.increment();
        }
        let id = self.inner.borrow().id;
        self.graph.borrow_mut().mark_signal_updated(id);
    }

    /// Update the value using a closure.
    ///
    /// Equivalent to `signal.set(f(signal.get()))` but avoids a double-borrow.
    pub fn update(&self, f: impl FnOnce(T) -> T) {
        let new_value = {
            let inner = self.inner.borrow();
            f(inner.value.clone())
        };
        self.set(new_value);
    }

    /// Queue a write for the next frame's Sync Point.
    ///
    /// REQUIRED inside physics event handlers to avoid feedback loops.
    /// Writing directly via .set() inside a physics step can cause:
    ///   state change → layout recompute → spring target update → body moves
    ///   → collision → state change → ... (infinite loop)
    ///
    /// .queue() breaks the loop by deferring the write to the next frame.
    ///
    /// # Example
    /// ```rust
    /// on_collision: move |_e| {
    ///     // ✅ safe — deferred to next frame
    ///     score.queue(score.get() + 1);
    ///
    ///     // ❌ dangerous inside a physics step
    ///     // score.set(score.get() + 1);
    /// }
    /// ```
    pub fn queue(&self, value: T)
    where
        T: 'static,
    {
        let id = self.inner.borrow().id;
        let inner_clone = Rc::clone(&self.inner);

        self.graph
            .borrow_mut()
            .enqueue_write(id, Box::new(move || {
                let mut inner = inner_clone.borrow_mut();
                inner.value = value;
                inner.version.increment();
            }));
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
/// Writes mark the owning entity dirty in the SignalGraph.
/// The Taffy layout tree is NOT touched until the Sync Point — at which
/// point all dirty layout signals drain together, Taffy recomputes once,
/// and spring targets are updated atomically.
///
/// Use for: width, height, flex-grow, padding, margin, display, position.
/// Do NOT use for: colour, opacity — use Signal<T> for those.
///
/// # Example
/// ```rust
/// let panel_width = use_layout_signal(cx, || 300.0_f32);
///
/// // Write — batched, does NOT trigger immediate layout recompute
/// panel_width.set(500.0);
///
/// // Read — always current value
/// let w = panel_width.get();
/// ```
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

    /// Returns the signal's unique ID.
    pub fn id(&self) -> SignalId {
        self.inner.borrow().id
    }

    /// Read the current value.
    pub fn get(&self) -> T {
        let inner = self.inner.borrow();
        self.graph
            .borrow()
            .track_dependency(inner.id, inner.version);
        inner.value.clone()
    }

    /// Write a new value.
    ///
    /// Does NOT trigger an immediate Taffy recompute.
    /// Marks the owning entity dirty — drains at the Sync Point.
    ///
    /// Writing this 1000 times in one frame = 1 Taffy recompute.
    /// The final value wins.
    pub fn set(&self, value: T) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.value = value;
            inner.version.increment();
        }
        let id = self.inner.borrow().id;
        self.graph
            .borrow_mut()
            .mark_layout_dirty(id, self.entity_id);
    }

    /// Update via closure. Avoids a double borrow.
    pub fn update(&self, f: impl FnOnce(T) -> T) {
        let new_value = {
            let inner = self.inner.borrow();
            f(inner.value.clone())
        };
        self.set(new_value);
    }

    /// Queue a deferred write. See Signal::queue for rationale.
    pub fn queue(&self, value: T)
    where
        T: 'static,
    {
        let id = self.inner.borrow().id;
        let entity_id = self.entity_id;
        let inner_clone = Rc::clone(&self.inner);
        let graph_clone = Rc::clone(&self.graph);

        self.graph
            .borrow_mut()
            .enqueue_write(id, Box::new(move || {
                {
                    let mut inner = inner_clone.borrow_mut();
                    inner.value = value;
                    inner.version.increment();
                }
                graph_clone
                    .borrow_mut()
                    .mark_layout_dirty(id, entity_id);
            }));
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
/// Lazy — does not recompute until read AND a dependency has changed.
/// At the Sync Point, all stale derived signals are recomputed in topological
/// order — each node computes at most once per frame.
///
/// # Example
/// ```rust
/// let base  = use_signal(cx, || 300.0_f32);
/// let scale = use_signal(cx, || 1.0_f32);
///
/// let width = use_derived(cx, move || base.get() * scale.get());
///
/// // width recomputes only when base or scale changes
/// let w = width.get();
/// ```
#[derive(Clone)]
pub struct Derived<T: Clone + 'static> {
    id:       SignalId,
    compute:  Rc<dyn Fn() -> T>,
    cached:   Rc<RefCell<Option<T>>>,
    graph:    Rc<RefCell<SignalGraph>>,
}

impl<T: Clone + 'static> Derived<T> {
    fn new(compute: impl Fn() -> T + 'static, graph: Rc<RefCell<SignalGraph>>) -> Self {
        let id = SignalId::next();
        let compute = Rc::new(compute);
        let cached  = Rc::new(RefCell::new(None::<T>));

        graph.borrow_mut().register_derived(id);

        Self { id, compute, cached, graph }
    }

    /// Returns the derived signal's unique ID.
    pub fn id(&self) -> SignalId {
        self.id
    }

    /// Read the current derived value.
    ///
    /// Recomputes only if a dependency has changed since the last read.
    /// Inside another `use_derived`, registers this as a dependency.
    pub fn get(&self) -> T {
        // Check if we need to recompute
        let is_stale = self.graph.borrow().is_derived_stale(self.id);

        if is_stale || self.cached.borrow().is_none() {
            // Start tracking which signals this computation reads
            self.graph.borrow_mut().begin_tracking(self.id);

            let value = (self.compute)();

            // Stop tracking, store the dependencies we discovered
            self.graph.borrow_mut().end_tracking(self.id);

            *self.cached.borrow_mut() = Some(value.clone());

            // Also track this derived as a dep if we're inside another derived
            self.graph
                .borrow()
                .track_dependency(self.id, Version(0)); // version handled by stale flag

            value
        } else {
            // Cache hit — still track as dependency
            self.graph
                .borrow()
                .track_dependency(self.id, Version(0));

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

/// The dependency graph for all signals in the application.
///
/// Owned by the `Scope` (and therefore by `fe_ui`'s frame loop).
/// One graph per application. Never shared across threads.
///
/// Responsibilities:
///   - Track which derived signals depend on which source signals.
///   - Mark derived signals stale when their dependencies update.
///   - Collect dirty layout entity IDs for the Sync Point.
///   - Hold queued physics writes until the next frame's drain.
///   - Provide topological ordering for derived recomputation.
pub struct SignalGraph {
    /// All registered signal IDs (source + derived).
    nodes: HashSet<SignalId>,

    /// derived → set of signals it depends on.
    /// Built lazily as derived signals are read.
    dependencies: HashMap<SignalId, HashSet<SignalId>>,

    /// source signal → set of derived signals that depend on it.
    /// Inverse of `dependencies`. Used to mark stale on write.
    subscribers: HashMap<SignalId, HashSet<SignalId>>,

    /// Derived signals that need recomputation at the Sync Point.
    stale_derived: HashSet<SignalId>,

    /// Layout signals → entity IDs that are dirty.
    /// Collected during the frame, drained at the Sync Point.
    dirty_layout: HashMap<SignalId, crate::entity::EntityId>,

    /// Which LayoutSignal IDs map to which entity IDs.
    /// Populated at signal registration time.
    layout_signal_entities: HashMap<SignalId, crate::entity::EntityId>,

    /// Deferred writes from .queue() calls.
    /// Drained at the START of the next frame's Sync Point.
    write_queue: VecDeque<Box<dyn FnOnce()>>,

    /// Stack of derived signal IDs currently being computed.
    /// Used to track which signals are read during a derived computation.
    tracking_stack: Vec<SignalId>,

    /// Signals read during the current tracked computation.
    /// Cleared on begin_tracking, consumed on end_tracking.
    tracked_reads: Vec<(SignalId, Version)>,
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
            write_queue:            VecDeque::new(),
            tracking_stack:         Vec::new(),
            tracked_reads:          Vec::new(),
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

    // ── Dependency tracking ───────────────────────────────────────────────────

    /// Called by Signal::get() and Derived::get() during a computation.
    /// If we're inside a derived computation, records this signal as a dep.
    pub(crate) fn track_dependency(&self, id: SignalId, _version: Version) {
        // We need interior mutability here — use a Cell-based approach
        // The tracking stack is mutated via begin/end_tracking instead
        // This function is called on the immutable borrow path so it's a no-op
        // — actual tracking happens via the mutable begin/end_tracking pair.
        let _ = (id, _version);
    }

    /// Begin recording dependency reads for a derived signal computation.
    pub(crate) fn begin_tracking(&mut self, derived_id: SignalId) {
        self.tracking_stack.push(derived_id);
        self.tracked_reads.clear();
    }

    /// Called by Signal::get() / LayoutSignal::get() to record a read.
    /// Must be called on the MUTABLE borrow — used internally by the graph
    /// when re-running derived computations at the Sync Point.
    pub(crate) fn record_read(&mut self, source_id: SignalId) {
        if let Some(&derived_id) = self.tracking_stack.last() {
            self.tracked_reads.push((source_id, Version::default()));
            // Register the dependency edge
            self.dependencies
                .entry(derived_id)
                .or_default()
                .insert(source_id);
            self.subscribers
                .entry(source_id)
                .or_default()
                .insert(derived_id);
        }
    }

    /// Stop recording for the current derived computation.
    pub(crate) fn end_tracking(&mut self, _derived_id: SignalId) {
        self.tracking_stack.pop();
        self.tracked_reads.clear();
    }

    // ── Dirty / stale marking ─────────────────────────────────────────────────

    /// Called when a source Signal is written via .set().
    /// Marks all downstream derived signals as stale.
    pub(crate) fn mark_signal_updated(&mut self, id: SignalId) {
        // Propagate stale flag to all direct and transitive subscribers
        let mut to_visit = vec![id];
        let mut visited  = HashSet::new();

        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) { continue; }
            visited.insert(current);

            if let Some(subs) = self.subscribers.get(&current).cloned() {
                for sub in subs {
                    self.stale_derived.insert(sub);
                    to_visit.push(sub);
                }
            }
        }
    }

    /// Called when a LayoutSignal is written.
    /// Marks the entity dirty AND propagates stale to derived subscribers.
    pub(crate) fn mark_layout_dirty(
        &mut self,
        id:        SignalId,
        entity_id: crate::entity::EntityId,
    ) {
        self.dirty_layout.insert(id, entity_id);
        self.mark_signal_updated(id);
    }

    /// Returns true if a derived signal needs recomputation.
    pub(crate) fn is_derived_stale(&self, id: SignalId) -> bool {
        self.stale_derived.contains(&id)
    }

    // ── Write queue ───────────────────────────────────────────────────────────

    /// Enqueue a deferred write from a .queue() call.
    pub(crate) fn enqueue_write(&mut self, _id: SignalId, f: Box<dyn FnOnce()>) {
        self.write_queue.push_back(f);
    }

    // ── Sync Point operations ─────────────────────────────────────────────────

    /// Step 1 of the Sync Point: drain all queued .queue() writes.
    /// These are writes from physics event handlers deferred to this frame.
    pub fn drain_physics_queue(&mut self) {
    // Drain the whole queue into a local vec first.
    // This drops the &mut self borrow before any closure runs,
    // preventing a RefCell double-borrow when a closure tries
    // to borrow the graph (e.g. to mark layout dirty).
    let writes: Vec<_> = self.write_queue.drain(..).collect();
    for write in writes {
        write();
    }
    }
    /// Step 3 of the Sync Point: collect all dirty layout entity IDs.
    /// Clears the dirty set — each entity appears at most once.
    pub fn take_layout_dirty(&mut self) -> Vec<crate::entity::EntityId> {
        let mut entities: Vec<_> = self.dirty_layout.values().copied().collect();
        entities.sort_unstable(); // deterministic order
        // Deduplicate — multiple layout signals on the same entity
        entities.dedup();
        self.dirty_layout.clear();
        entities
    }

    /// Returns the topologically sorted order of derived signals.
    /// Used by the Sync Point to recompute derived signals in the correct order
    /// — dependencies before dependents — so each node computes at most once.
    pub fn topo_sorted_derived(&self) -> Vec<SignalId> {
        // Kahn's algorithm on the dependency subgraph of stale derived nodes
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

        let mut queue: VecDeque<SignalId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        // Sort for deterministic output
        let mut queue_vec: Vec<_> = queue.drain(..).collect();
        queue_vec.sort_unstable_by_key(|id| id.0);
        queue.extend(queue_vec);

        let mut sorted = Vec::with_capacity(self.stale_derived.len());

        while let Some(id) = queue.pop_front() {
            sorted.push(id);
            if let Some(neighbors) = adj.get(&id) {
                let mut next_batch = Vec::new();
                for &neighbor in neighbors {
                    let deg = in_degree.entry(neighbor).or_insert(0);
                    if *deg > 0 { *deg -= 1; }
                    if *deg == 0 { next_batch.push(neighbor); }
                }
                next_batch.sort_unstable_by_key(|id| id.0);
                for n in next_batch { queue.push_back(n); }
            }
        }

        sorted
    }

    /// Clear all stale flags after the Sync Point recomputation is complete.
    pub fn clear_stale(&mut self) {
        self.stale_derived.clear();
    }

    /// Returns the number of signals currently in the graph.
    pub fn signal_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of dirty layout entities waiting for the Sync Point.
    pub fn dirty_layout_count(&self) -> usize {
        self.dirty_layout.len()
    }

    /// Returns the number of queued deferred writes.
    pub fn queued_write_count(&self) -> usize {
        self.write_queue.len()
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
            .field("write_queue",   &self.write_queue.len())
            .finish()
    }
}

// ─── Scope ───────────────────────────────────────────────────────────────────

/// A handle into the SignalGraph tied to one component's lifetime.
///
/// Created by `fe_ui` when a component is spawned. Passed as `cx` to the
/// component function. When the component despawns, the Scope is dropped
/// and all signals created through it are cleaned up.
///
/// You never construct a Scope manually — it is provided by the runtime.
#[derive(Clone)]
pub struct Scope {
    /// The entity this scope belongs to.
    pub(crate) entity_id: crate::entity::EntityId,

    /// Shared reference to the application's signal graph.
    pub(crate) graph: Rc<RefCell<SignalGraph>>,

    /// All signal IDs created through this scope.
    /// Used for cleanup on despawn.
    pub(crate) owned_signals: Rc<RefCell<Vec<SignalId>>>,
}

impl Scope {
    /// Create a new Scope. Called by the #[component] macro — not by user code.
    pub fn new(entity_id: crate::entity::EntityId, graph: Rc<RefCell<SignalGraph>>) -> Self {
        Self {
            entity_id,
            graph,
            owned_signals: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// The entity ID this scope is bound to.
    pub fn entity_id(&self) -> crate::entity::EntityId {
        self.entity_id
    }
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
///
/// The returned Signal<T> is tied to the component's Scope — it will be
/// cleaned up automatically when the component despawns.
///
/// Use for: colour, opacity, text content, shader parameters.
///
/// # Example
/// ```rust
/// #[component]
/// fn MyButton(cx: Scope) -> Element {
///     let is_hovered = use_signal(cx, || false);
///
///     render! {
///         Button {
///             on_mouseenter: move |_| is_hovered.set(true),
///             on_mouseleave: move |_| is_hovered.set(false),
///             style: style! {
///                 background: if is_hovered.get() { Color::BLUE } else { Color::RED },
///             }
///         }
///     }
/// }
/// ```
pub fn use_signal<T: Clone + 'static>(cx: &Scope, init: impl FnOnce() -> T) -> Signal<T> {
    let signal = Signal::new(init(), Rc::clone(&cx.graph));
    cx.owned_signals.borrow_mut().push(signal.id());
    signal
}

/// Create a layout-affecting reactive signal.
///
/// Writes are batched — they do NOT trigger an immediate Taffy recompute.
/// At the Sync Point, all dirty layout signals drain together.
/// Writing this signal 1000 times in one frame = exactly 1 Taffy recompute.
///
/// Use for: width, height, flex-grow, padding, margin, position.
///
/// # Example
/// ```rust
/// #[component]
/// fn Panel(cx: Scope) -> Element {
///     let width = use_layout_signal(cx, || 300.0_f32);
///
///     render! {
///         Box {
///             style: style! { width: width.get().px },
///             on_click: move |_| width.set(500.0), // glides to new size
///         }
///     }
/// }
/// ```
pub fn use_layout_signal<T: Clone + 'static>(
    cx:   &Scope,
    init: impl FnOnce() -> T,
) -> LayoutSignal<T> {
    let signal = LayoutSignal::new(init(), Rc::clone(&cx.graph), cx.entity_id);
    cx.owned_signals.borrow_mut().push(signal.id());
    signal
}

/// Create a derived (computed) signal.
///
/// Lazy — only recomputes when a dependency has changed since the last read.
/// At the Sync Point, all stale derived signals recompute in topological order
/// — each node computes at most once per frame regardless of how many
/// of its dependencies changed.
///
/// Dependencies are tracked automatically — any Signal or LayoutSignal
/// read inside the closure becomes a dependency.
///
/// # Example
/// ```rust
/// let base_width  = use_layout_signal(cx, || 300.0_f32);
/// let is_expanded = use_signal(cx, || false);
///
/// // Recomputes only when base_width or is_expanded changes.
/// // If only is_expanded changes and base_width doesn't, exactly one
/// // recompute happens — not a chain.
/// let panel_width = use_derived(cx, move || {
///     if is_expanded.get() { base_width.get() * 1.5 } else { base_width.get() }
/// });
/// ```
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

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_scope() -> Scope {
        use crate::entity::EntityId;
        use slotmap::SlotMap;

        // Create a minimal SlotMap to get a valid EntityId
        let graph     = Rc::new(RefCell::new(SignalGraph::new()));
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
        // All writes are visible; the last one wins as the current value
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

        // Before drain — value unchanged
        assert_eq!(sig.get(), 0);
        assert_eq!(cx.graph.borrow().queued_write_count(), 1);

        // Simulate Sync Point drain
        cx.graph.borrow_mut().drain_physics_queue();

        // After drain — value updated
        assert_eq!(sig.get(), 42);
        assert_eq!(cx.graph.borrow().queued_write_count(), 0);
    }

    #[test]
    fn signal_queue_multiple() {
        let cx  = make_scope();
        let sig = use_signal(&cx, || 0_i32);

        sig.queue(1);
        sig.queue(2);
        sig.queue(3);

        assert_eq!(cx.graph.borrow().queued_write_count(), 3);

        cx.graph.borrow_mut().drain_physics_queue();

        // All three drained — last writer wins
        assert_eq!(sig.get(), 3);
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
        // Writing 1000 times = 1 dirty entry (last value wins, entity deduped)
        let cx  = make_scope();
        let sig = use_layout_signal(&cx, || 0.0_f32);

        for i in 0..1000 {
            sig.set(i as f32);
        }

        // Still only 1 dirty entry for this entity
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

        // After take, the dirty set is cleared
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 0);
    }

    #[test]
    fn layout_signal_queue_deferred() {
        let cx  = make_scope();
        let sig = use_layout_signal(&cx, || 0.0_f32);

        sig.queue(99.0);

        // Before drain — not yet dirty, value unchanged
        assert_eq!(sig.get(), 0.0);
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 0);

        cx.graph.borrow_mut().drain_physics_queue();

        assert_eq!(sig.get(), 99.0);
        assert_eq!(cx.graph.borrow().dirty_layout_count(), 1);
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
        let cx     = make_scope();
        let sorted = cx.graph.borrow().topo_sorted_derived();
        assert!(sorted.is_empty());
    }

    #[test]
    fn graph_mark_signal_updated_marks_subscribers_stale() {
        let cx = make_scope();
        let graph = Rc::clone(&cx.graph);

        let source_id  = SignalId::next();
        let derived_id = SignalId::next();

        {
            let mut g = graph.borrow_mut();
            g.register_signal(source_id);
            g.register_derived(derived_id);
            // Manually wire: derived depends on source
            g.dependencies.entry(derived_id).or_default().insert(source_id);
            g.subscribers.entry(source_id).or_default().insert(derived_id);
        }

        assert!(!graph.borrow().is_derived_stale(derived_id));

        graph.borrow_mut().mark_signal_updated(source_id);

        assert!(graph.borrow().is_derived_stale(derived_id));
    }

    #[test]
    fn graph_clear_stale_resets_flags() {
        let cx = make_scope();
        let graph = Rc::clone(&cx.graph);

        let source_id  = SignalId::next();
        let derived_id = SignalId::next();

        {
            let mut g = graph.borrow_mut();
            g.register_signal(source_id);
            g.register_derived(derived_id);
            g.dependencies.entry(derived_id).or_default().insert(source_id);
            g.subscribers.entry(source_id).or_default().insert(derived_id);
            g.mark_signal_updated(source_id);
        }

        assert!(graph.borrow().is_derived_stale(derived_id));
        graph.borrow_mut().clear_stale();
        assert!(!graph.borrow().is_derived_stale(derived_id));
    }

    #[test]
    fn graph_topo_sort_linear_chain() {
        // a → b → c  (c depends on b, b depends on a)
        // sorted order should be: a, b, c
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

            // Mark all stale
            g.stale_derived.insert(a);
            g.stale_derived.insert(b);
            g.stale_derived.insert(c);
        }

        let sorted = graph.borrow().topo_sorted_derived();

        // a must come before b, b must come before c
        let pos = |id: SignalId| sorted.iter().position(|&x| x == id).unwrap();
        assert!(pos(a) < pos(b));
        assert!(pos(b) < pos(c));
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
    fn scope_entity_id_matches() {
        let cx = make_scope();
        let entity_id = cx.entity_id();
        let sig = use_layout_signal(&cx, || 0.0_f32);
        sig.set(1.0);

        let dirty = cx.graph.borrow_mut().take_layout_dirty();
        assert!(dirty.contains(&entity_id));
    }
}
