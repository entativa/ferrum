# Ferrum `fe_ui` — Architecture

> This document is for contributors and advanced users. It describes how the crates are structured, how they communicate, and where the hard abstraction boundaries are.
>
> If you find yourself needing to reach past these boundaries to build something, open an issue. It means the public API has a gap.

---

## The Core Principle: Shared Language, Isolated Systems

Ferrum runs three independent subsystems simultaneously — layout (Taffy), physics (Rapier), rendering (wgpu). Each subsystem speaks its own internal language. None of them know about each other.

They communicate exclusively through **`fe_ui_core` types** — a shared vocabulary that every crate depends on, but nothing else does.

```
                        ┌─────────────────┐
                        │   fe_ui_core    │
                        │                 │
                        │  LayoutRect     │
                        │  Transform2D    │
                        │  EntityId       │
                        │  Signal<T>      │
                        │  PhysicsState   │
                        │  GhostWorld     │
                        └────────┬────────┘
                                 │  all crates depend on core
                                 │  no crate depends on another subsystem
              ┌──────────────────┼──────────────────┐
              │                  │                  │
              ▼                  ▼                  ▼
      ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
      │ fe_ui_taffy  │  │ fe_ui_rapier │  │  fe_ui_wgpu  │
      │              │  │              │  │              │
      │  Layout      │  │  Physics     │  │  Rendering   │
      │  computation │  │  simulation  │  │  pipeline    │
      └──────────────┘  └──────────────┘  └──────────────┘
              │                  │                  │
              └──────────────────┼──────────────────┘
                                 │  all subsystems report into
                                 ▼
                        ┌─────────────────┐
                        │     fe_ui       │
                        │                 │
                        │  Frame loop     │
                        │  Sync Point     │
                        │  Orchestration  │
                        └─────────────────┘
                                 │
                        ┌────────┴────────┐
                        │     fe_std      │
                        │                 │
                        │  Button         │
                        │  Modal          │
                        │  ScrollArea     │
                        │  ...            │
                        └─────────────────┘
```

**The dependency rule**: arrows point downward only. `fe_ui_taffy` knows about `fe_ui_core`. It does not know about `fe_ui_rapier` or `fe_ui_wgpu`. This is enforced — a PR that adds a cross-subsystem dependency is rejected.

---

## Crate Breakdown

### `fe_ui_core` — The Shared Language

No GPU. No physics. No layout engine. Only types.

```
fe_ui_core/
├── src/
│   ├── entity.rs       # EntityId, GhostWorld (the internal ECS)
│   ├── types.rs        # LayoutRect, Transform2D, Vec2, Color, Bounds
│   ├── signal.rs       # Signal<T>, use_signal, use_layout_signal, use_derived
│   ├── physics.rs      # PhysicsState (Active | Settling | Asleep), PhysicsProps
│   ├── style.rs        # Style, Flex, Grid, Dimension types
│   └── lib.rs
```

**Compile time target**: `< 3s` from clean. No proc-macros. No heavy dependencies. This crate must stay lean — everything else depends on it.

**Key types**:

```rust
// The only shared identity across all three subsystems
pub struct EntityId(u32);

// What Taffy produces. What Rapier consumes as spring target.
// What wgpu uses to size the SDF quad.
pub struct LayoutRect {
    pub x:      f32,
    pub y:      f32,
    pub width:  f32,
    pub height: f32,
}

// What Rapier produces. What wgpu consumes for draw position.
pub struct Transform2D {
    pub position: Vec2,
    pub rotation: f32,
    pub scale:    Vec2,
}

// Observable physics state — exposed to developers via widget.physics_state()
pub enum PhysicsState {
    Active,     // body is being stepped, forces applied
    Settling,   // within settle threshold, decelerating
    Asleep,     // at target, zero cost
}
```

---

### `fe_ui_core` — The Ghost ECS

Ferrum runs an internal ECS. You will never interact with it directly.

It exists because three independent systems — Taffy, Rapier, wgpu — each need to track the same set of entities. Taffy assigns a `NodeId`. Rapier assigns a `RigidBodyHandle`. wgpu assigns an instance buffer index. Something must own the mapping between all three. That something is the `GhostWorld`.

```rust
// fe_ui_core/src/entity.rs
// Internal only. Never pub outside fe_ui_core.

pub(crate) struct GhostEntity {
    pub id:          EntityId,
    pub taffy_node:  taffy::NodeId,
    pub rapier_body: rapier2d::RigidBodyHandle,
    pub wgpu_index:  u32,              // index into the SDF instance buffer
    pub physics:     PhysicsProps,
    pub style:       Style,
    pub children:    Vec<EntityId>,
    pub parent:      Option<EntityId>,
    pub joint:       JointType,        // Fixed | Spring | Free
}

pub(crate) struct GhostWorld {
    entities: SlotMap<EntityId, GhostEntity>,
}

impl GhostWorld {
    pub(crate) fn spawn(&mut self, props: SpawnProps) -> EntityId { ... }
    pub(crate) fn despawn(&mut self, id: EntityId) { ... }
    pub(crate) fn get(&self, id: EntityId) -> Option<&GhostEntity> { ... }
    pub(crate) fn iter_active(&self) -> impl Iterator<Item = &GhostEntity> { ... }
}
```

**The Ghost ECS as a design pressure**: Every time a developer needs to reach into entity storage to build something, it signals a gap in the `#[component]` macro or the `fe_std` API. The Ghost ECS is never the answer to a user-facing problem — fixing the API is.

> "If you find yourself needing `GhostWorld` access, open an issue. The component API has a gap."

---

### `fe_ui_taffy` — Layout

Wraps Taffy. Owns the layout tree. Produces `LayoutRect` outputs that the rest of the system consumes.

```
fe_ui_taffy/
├── src/
│   ├── tree.rs         # LayoutTree — wraps taffy::TaffyTree
│   ├── cache.rs        # Layout result cache — invalidated by dirty signals
│   ├── convert.rs      # fe_ui Style → taffy Style conversion
│   └── lib.rs
```

**Responsibilities**:
- Maintain a Taffy node tree mirroring the `GhostWorld` entity tree
- On Sync Point: receive dirty `EntityId` set from signal system, recompute only affected subtrees
- Output: `HashMap<EntityId, LayoutRect>` — the resolved target positions for every entity

**What it does not do**:
- It does not know about Rapier. It produces rects. Someone else injects them into springs.
- It does not know about wgpu. It produces rects. Someone else sizes quads.
- It does not own signals. It receives a dirty set and a style map. It computes.

```rust
// fe_ui_taffy public surface (internal to fe_ui only)
pub struct LayoutTree { ... }

impl LayoutTree {
    pub fn compute(&mut self, dirty: &[EntityId], styles: &StyleMap)
        -> HashMap<EntityId, LayoutRect>;
}
```

---

### `fe_ui_rapier` — Physics

Wraps Rapier2D. Owns the physics world. Produces `Transform2D` outputs that wgpu consumes.

```
fe_ui_rapier/
├── src/
│   ├── world.rs        # PhysicsWorld — wraps rapier2d::RigidBodySet et al.
│   ├── solver.rs       # Spring constraint solver — the layout↔physics bridge
│   ├── gesture.rs      # Gesture velocity tracking → impulse injection
│   ├── joints.rs       # Fixed, Spring, Free joint implementations
│   └── lib.rs
```

**Responsibilities**:
- Maintain Rapier rigid bodies and colliders for every entity
- On Sync Point: receive new `LayoutRect` targets, update spring constraint goals
- Step the physics world at 240Hz with substeps
- Output: `HashMap<EntityId, (Transform2D, PhysicsState, Vec2)>` — position, state, velocity

**The spring constraint solver** (`solver.rs`) is the most critical file in the codebase. It is the bridge between Taffy's "where things should be" and Rapier's "where things are."

```rust
// solver.rs — simplified
pub fn update_spring_targets(
    bodies:  &mut RigidBodySet,
    joints:  &mut ImpulseJointSet,
    targets: &HashMap<EntityId, LayoutRect>,  // from Taffy via Sync Point
    handles: &HashMap<EntityId, RigidBodyHandle>,
) {
    for (id, target) in targets {
        let handle = handles[id];
        let body   = &mut bodies[handle];

        // Spring motor seeks the Taffy target position
        // The body never teleports. It always arrives via force.
        set_spring_target(body, target.center(), target.size());
    }
}
```

**What it does not do**:
- It does not know about Taffy directly. It receives `LayoutRect` values. It doesn't know where they came from.
- It does not know about wgpu. It produces transforms. Someone else draws them.
- It does not own signals. Physics events are queued as signals for the next frame.

---

### `fe_ui_wgpu` — Rendering

Owns the wgpu device, surface, and render pipeline. Consumes `Transform2D` from Rapier (interpolated) and `LayoutRect` from Taffy (for quad sizing). Produces pixels.

```
fe_ui_wgpu/
├── src/
│   ├── pipeline.rs     # wgpu render pipeline setup
│   ├── instance.rs     # SDF instance buffer — packing and upload
│   ├── interpolate.rs  # Physics interpolation (prev/curr transform → render transform)
│   ├── atlas.rs        # Glyph atlas — LRU eviction, SDF glyph rasterization
│   ├── portal.rs       # Stencil mask generation for Portal clipping
│   └── lib.rs
├── shaders/
│   ├── sdf_uber.wgsl   # The Iron-Sight uber-shader
│   └── text.wgsl       # Text-specific SDF pass
```

**Responsibilities**:
- Interpolate physics transforms between steps (alpha-based, see Rendering Pipeline doc)
- Pack all Tier 0 entities into a single `SdfInstance` buffer
- Execute the three-pass draw sequence (portal stencil → SDF batch → custom materials)
- Manage the glyph atlas for text rendering

**What it does not do**:
- It does not run physics. It consumes transforms.
- It does not run layout. It consumes rects for quad sizing.
- It does not own the frame loop. It exposes a `render(frame: &FrameData)` function.

---

### `fe_ui` — The Orchestrator

The top-level crate. Owns the frame loop, the Sync Point, and the event system. The only crate that imports all three subsystems.

```
fe_ui/
├── src/
│   ├── app.rs          # App builder — window config, run()
│   ├── frame.rs        # The frame loop — event → sync → physics → render
│   ├── sync.rs         # The Sync Point implementation
│   ├── event.rs        # Input event routing → signal writes or impulses
│   ├── macros.rs       # #[component] and render! macro definitions
│   └── prelude.rs      # pub use everything a developer needs
```

**The frame loop** (`frame.rs`) is the single source of truth for execution order:

```rust
// fe_ui/src/frame.rs
pub fn run_frame(
    app:     &mut AppState,
    layout:  &mut LayoutTree,     // fe_ui_taffy
    physics: &mut PhysicsWorld,   // fe_ui_rapier
    renderer: &mut Renderer,      // fe_ui_wgpu
    signals: &mut SignalGraph,
) {
    // 1. Process input events
    //    → immediate signals (.set()) fire here
    //    → impulses queued for physics injection
    app.drain_events(signals);

    // ─── SYNC POINT ──────────────────────────────────────────
    // 2. Drain queued signals from previous frame's physics events
    signals.drain_physics_queue();

    // 3. Recompute dirty derived signals (topological order)
    signals.resolve_derived();

    // 4. Collect dirty layout nodes
    let dirty = signals.take_layout_dirty();

    // 5. Taffy recomputes affected subtrees — once, atomically
    let new_targets = layout.compute(&dirty, &app.styles);

    // 6. Diff against previous targets — only changed nodes update springs
    let changed_targets = diff_targets(&app.prev_targets, &new_targets);

    // 7. Inject new spring targets into Rapier constraints
    physics.update_spring_targets(&changed_targets);
    // ─────────────────────────────────────────────────────────

    // 8. Inject queued impulses from input events
    physics.drain_impulse_queue();

    // 9. Step physics (240Hz internal, substeps × 4)
    //    Collision events → queued as signals for next frame
    physics.step(signals);

    // 10. Render
    //     Interpolate transforms, pack instance buffer, draw
    renderer.render(&physics.transforms(), &new_targets);
}
```

Every frame. In this order. No exceptions.

---

### `fe_std` — Standard Components

Depends only on `fe_ui` (the prelude). Uses only the public API. If `fe_std` needs internal access to a subsystem, the public API has a gap.

```
fe_std/
├── src/
│   ├── button.rs
│   ├── switch.rs
│   ├── slider.rs
│   ├── scroll_area.rs
│   ├── modal.rs
│   ├── toast.rs
│   ├── drawer.rs
│   ├── tab_bar.rs
│   └── lib.rs
```

`fe_std` is a proof of concept for the public API. If building any component requires workarounds, the workaround belongs in `fe_ui`, not in `fe_std`.

---

## The `#[component]` Macro

The macro is the only public door into the Ghost ECS. It handles all three subsystem registrations invisibly.

```rust
// What the developer writes
#[component]
pub fn BouncingButton(cx: Scope, label: String) -> Element {
    render! {
        RigidBody {
            physics: Physics::dynamic().mass(2.0).restitution(0.8),
            style: style! { width: 200.px, height: 60.px },
            Text { "{label}" }
        }
    }
}

// What the macro generates (simplified)
pub fn BouncingButton(cx: Scope, label: String) -> Element {
    let entity_id = cx.ghost_world().spawn(SpawnProps {
        style:   Style { width: Dimension::Px(200.0), height: Dimension::Px(60.0) },
        physics: PhysicsProps { mass: 2.0, restitution: 0.8, ..dynamic_defaults() },
        joint:   JointType::Fixed,  // default — no Physics::dynamic() on parent
    });

    cx.layout_tree().register(entity_id, taffy_style);
    cx.physics_world().spawn_body(entity_id, physics_props);
    cx.renderer().reserve_instance(entity_id);

    // Returns an Element handle — the developer never sees EntityId
    Element::from_entity(entity_id)
}
```

The developer writes `RigidBody { physics: ..., style: ... }`. The macro handles Taffy registration, Rapier body spawning, and wgpu instance reservation. Three systems, one declaration, zero boilerplate.

---

## Data Flow Summary

```
USER INPUT
    │
    ▼
[Event System]  →  use_signal.set()       →  GPU uniform (next render)
                →  use_layout_signal.set() →  dirty set (next Sync Point)
                →  impulse queue           →  physics injection (next step)
                →  physics event           →  signal.queue() (next frame)
                │
                ▼
         ═══ SYNC POINT ═══
                │
                ▼
[Signal Graph]  →  resolve derived signals (topo-sorted)
                →  collect layout dirty set
                │
                ▼
[fe_ui_taffy]   →  recompute dirty subtrees
                →  output: HashMap<EntityId, LayoutRect>
                │
                ▼
[fe_ui_rapier]  →  diff targets, update spring goals
                →  inject input impulses
                →  step world × substeps
                →  output: HashMap<EntityId, (Transform2D, PhysicsState, Vec2)>
                │
                ▼
[fe_ui_wgpu]    →  interpolate transforms (alpha)
                →  pack SDF instance buffer
                →  draw (portal stencil → Tier 0 batch → Tier 1 custom)
                │
                ▼
              PIXELS
```

---

## Contribution Rules

These are not guidelines. They are invariants. A PR that violates them is not merged.

1. **No cross-subsystem imports**: `fe_ui_taffy` does not import `fe_ui_rapier`. `fe_ui_rapier` does not import `fe_ui_wgpu`. They speak `fe_ui_core` only.
2. **No public `EntityId`**: `EntityId` is never exposed in a public API. If a user needs to identify a widget, they use a signal or a callback.
3. **No GhostWorld access outside macros**: `GhostWorld` is `pub(crate)` in `fe_ui_core`. The `#[component]` macro is the only door in.
4. **Frame loop order is sacred**: The execution order in `frame.rs` does not change without an RFC. Reordering steps has non-obvious physics and rendering consequences.
5. **`fe_ui_core` stays lean**: No new heavy dependencies in `fe_ui_core`. It must compile in `< 3s`. Every dependency added is a dependency every downstream crate pays for.
6. **`fe_std` uses only the public API**: If building a standard component requires internal access, fix the public API first.

---

## File Index

| File | Crate | Purpose |
|---|---|---|
| `entity.rs` | `fe_ui_core` | GhostEntity, GhostWorld, EntityId |
| `signal.rs` | `fe_ui_core` | Signal<T>, SignalGraph, dirty tracking |
| `solver.rs` | `fe_ui_rapier` | Spring constraint solver — layout↔physics bridge |
| `frame.rs` | `fe_ui` | The frame loop — execution order |
| `sync.rs` | `fe_ui` | The Sync Point implementation |
| `sdf_uber.wgsl` | `fe_ui_wgpu` | The Iron-Sight uber-shader |
| `interpolate.rs` | `fe_ui_wgpu` | Physics interpolation (alpha-based) |
| `macros.rs` | `fe_ui` | #[component] and render! |

---

*See also: [Signal Graph](./SIGNAL_GRAPH.md) — the Sync Point in detail.*
*See also: [Rendering Pipeline](./RENDERING_PIPELINE.md) — what fe_ui_wgpu does with the transforms.*
*See also: [Scene Spec](./SCENE_SPEC.md) — how the PhysicsWorld is configured.*
