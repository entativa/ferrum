# Ferrum `fe_ui` — The Signal Graph

> **Signals are Intent. Physics is Reality. The Sync Point is where Intent becomes Reality.**

---

## The Problem: Three Loops, One Screen

A Ferrum application runs three concurrent loops at all times:

| Loop | Rate | Responsibility |
|---|---|---|
| **Signal Graph** | Event-driven | Reactive state — fires on change |
| **Physics Loop** | 240Hz fixed | Deterministic simulation — Rapier steps |
| **Render Loop** | 60 / 144Hz | Interpolated output — wgpu draws |

These loops cannot freely write into each other. If they do, three failure modes emerge:

**Zone 1 — Layout Thrash**: A signal update triggers a Taffy recompute mid-physics-step. The body receives a new spring target while already in motion. The spring force spikes. The widget visibly jumps.

**Zone 2 — Cascading Updates**: One signal change triggers three derived signals, each triggering a layout recompute in the same frame. Taffy runs four times when it should run once.

**Zone 3 — The Feedback Loop**: A physics collision updates a signal → signal updates a style → style updates a layout target → target moves the body → body collides again. Infinite loop. App locks.

The solution to all three is **The Sync Point**.

---

## The Sync Point

One enforced boundary per frame. Signals drain into the physics world exactly once — at the end of the event loop, before the physics step begins.

```
┌─────────────────────────────────────────────────────┐
│                    FRAME N                          │
│                                                     │
│  [Event Loop]                                       │
│    User input → signals fire                        │
│    Derived signals compute                          │
│    use_layout_signal nodes marked dirty             │
│                          │                          │
│                          ▼                          │
│              ┌─── SYNC POINT ───┐                   │
│              │                 │                   │
│              │  1. Collect all │                   │
│              │     dirty nodes │                   │
│              │  2. Commit to   │                   │
│              │     Taffy       │                   │
│              │  3. Resolve     │                   │
│              │     rects       │                   │
│              │  4. Inject new  │                   │
│              │     target_pos  │                   │
│              │     → springs   │                   │
│              │                 │                   │
│              └────────┬────────┘                   │
│                       │                            │
│                       ▼                            │
│  [Physics Step × substeps]                         │
│    Rapier steps at 240Hz                           │
│    Bodies seek new spring targets                  │
│    Collision events → queued for FRAME N+1         │
│                       │                            │
│                       ▼                            │
│  [Render]                                          │
│    wgpu draws SDF quads at interpolated positions  │
│                                                    │
└─────────────────────────────────────────────────────┘
```

The physics world never knows about business logic. It only knows that a goal post (the Taffy rect) has moved, and it needs to use spring force to get the body there.

---

## One-Way Data Flow — Enforced Per Step

```
Signals ──► Taffy ──► Rapier ──► Render
                                   │
                    ┌──────────────┘
                    │  Collision / physics events
                    │  queued as signals for FRAME N+1
                    ▼
                 Signal Queue
                    │
                    ▼
              FRAME N+1 Sync Point
```

**No signal writes back into the current physics step.** If a physics event needs to update application state, it queues. The queue drains at the next Sync Point.

---

## The Signal Types

Ferrum exposes two signal primitives. The distinction between them *is* the Sync Point contract.

### `use_signal` — Immediate, GPU-only

For state that affects appearance but not layout. Fires immediately. Updates a material uniform on the GPU. The physics simulation is completely unaware.

```rust
let is_hovered = use_signal(cx, || false);

// When this fires:
// → GPU material uniform updated next render frame
// → Taffy tree unchanged
// → Rapier bodies unchanged
// → Physics running uninterrupted at 240Hz
on_mouseenter: move |_| is_hovered.set(true),
```

Use for: colors, opacity, shader parameters, visibility toggles, text content.

### `use_layout_signal` — Batched, layout-affecting

For state that affects size, position, or flex properties. Does **not** trigger an immediate recompute. Marks the node as dirty in the Taffy buffer. Drains at the next Sync Point.

```rust
let panel_width = use_layout_signal(cx, || 300.0);

// When this fires:
// → Node marked dirty in layout buffer
// → Nothing else happens yet
// → At Sync Point: Taffy recomputes, spring target updates
// → Body glides to new position via spring force
on_click: move |_| panel_width.set(500.0),
```

Use for: width, height, flex-grow, padding, margin, display, position.

```rust
// In the style block — Ferrum knows this is layout-affecting
// because panel_width is a use_layout_signal
style: style! {
    width: panel_width.get().px,
    height: 200.px,
}
```

---

## The Reactive Drain

Signals can fire as many times as they want during a frame. Only the **final value** at drain time matters.

```rust
// All three of these fire in the same frame
panel_width.set(350.0);
panel_width.set(420.0);
panel_width.set(500.0);

// At the Sync Point, Taffy sees: 500.0
// Taffy recomputes exactly once.
// The spring target updates exactly once.
// Zero wasted layout work.
```

This is the **Reactive Drain** — the signal graph is treated as accumulated intent, collapsed to a single truth at the boundary.

---

## Zone 3 — Collision Events & The Queue

Physics events that need to update application state use `.queue()` instead of `.set()`. Queued writes are held until the next frame's Sync Point.

```rust
on_collision: move |e| {
    // ❌ WRONG — immediate write during physics step
    //    Risks feedback loop: state → layout → target → body → collision → repeat
    score.set(score.get() + 1);

    // ✅ RIGHT — queued for next frame's signal drain
    //    Physics step completes cleanly. Score updates next frame.
    score.queue(score.get() + 1);
},
```

The compiler does not enforce `.queue()` — but Ferrum will emit a **warning in debug builds** if a `use_layout_signal` is written via `.set()` inside a physics event handler. The inspector (Phase 3) will highlight the offending signal node in red.

---

## Derived Signals

Derived signals are computed values that depend on one or more source signals. They are lazy — they do not recompute until read, and only if a dependency has changed.

```rust
// Source signals
let base_width  = use_layout_signal(cx, || 300.0);
let is_expanded = use_signal(cx, || false);

// Derived — recomputes only when base_width or is_expanded changes
let panel_width = use_derived(cx, move || {
    if *is_expanded { *base_width * 1.5 } else { *base_width }
});
```

**Cascade prevention**: If `is_expanded` changes and `base_width` does not, only one recompute happens at the Sync Point — not a chain. The dependency graph is topologically sorted before drain. Each node computes at most once per frame.

---

## The Full Signal → Physics Contract

```
FRAME START
│
├─ [Input events processed]
│    signals fire via .set() or .queue()
│    use_layout_signal nodes → marked dirty (not yet committed)
│    use_signal nodes → GPU uniform scheduled (immediate, next render)
│
├─ [Derived signals lazily invalidated]
│    dependency graph marks stale nodes
│    no computation yet
│
├─── SYNC POINT ──────────────────────────────────────────────────
│    1. Collect all dirty use_layout_signal nodes
│    2. Recompute stale derived signals (topo-sorted, once each)
│    3. Commit final values → Taffy layout tree
│    4. Taffy resolves new target rects (one full pass)
│    5. Diff against previous rects → only changed nodes update springs
│    6. Spring target_pos updated in Rapier constraints
│    7. Queued signals (.queue()) from previous frame drained → .set()
│─────────────────────────────────────────────────────────────────
│
├─ [Physics Step]
│    Rapier substeps × 4 at 240Hz
│    Bodies seek spring targets with force
│    Collision events → .queue() into signal queue for FRAME N+1
│    Bodies that reach target within 0.1px + near-zero velocity → sleep
│
├─ [Render]
│    wgpu interpolates body positions between physics steps
│    SDF quads drawn at interpolated transforms
│    GPU uniforms from use_signal updates applied
│
FRAME END
```

---

## DX Summary

| Signal Type | When to use | Batched? | Physics-aware? |
|---|---|---|---|
| `use_signal` | Color, opacity, text, shader params | No — immediate | No |
| `use_layout_signal` | Width, height, flex, position | Yes — Sync Point | Yes — updates spring target |
| `use_derived` | Computed from other signals | Yes — lazy, topo-sorted | Depends on source type |
| `.set()` | Write from UI events | Immediate | Safe in event handlers |
| `.queue()` | Write from physics events | Next frame Sync Point | Required in physics handlers |

---

## The Sync Point in One Sentence

> Signals accumulate intent during the frame. At the Sync Point, intent collapses into a single layout truth. The physics world wakes up to a stable, consistent set of spring targets — and never sees the chaos that produced them.

---

*See also: [Scene & Component Spec](./SCENE_SPEC.md) — how `scene.rs` configures the physics world that the Signal Graph feeds into.*
