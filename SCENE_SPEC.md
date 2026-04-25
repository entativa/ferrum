# Ferrum `fe_ui` — Scene & Component Spec

> **The Iron Rule**: A component should never have to manually update its position. The developer defines the `Style` (Target) and the `Physics` (Behavior). The Constraint Solver handles the rest.

---

## `scene.rs` — Staging the World

`scene.rs` is where the engine meets the UI. Not a massive config struct — a **builder pattern** that reads like setting up a stage.

```rust
// src/scene.rs
use fe_ui::prelude::*;

pub fn setup_scene(world: &mut SceneWorld) {

    // 1. GLOBAL ENVIRONMENT
    // Setting the "vibe" of the entire application.
    world.set_environment(Environment {
        gravity:          Vec2::new(0.0, 9.81), // standard earth gravity
        air_density:      0.1,                  // global drag on all active bodies
        pixels_per_meter: 100.0,                // bridge between Taffy units and Rapier units
    });

    // 2. THE SIMULATION STEPPER
    // Physics Hz and render Hz are fully decoupled.
    // Ferrum interpolates body positions so the UI stays
    // smooth at 60/144fps regardless of the physics step rate.
    world.set_simulation(SimulationConfig {
        physics_hz:        240.0,
        substeps:          4,
        solver_iterations: 8,
    });

    // 3. GLOBAL SPRING DEFAULTS
    // When a widget reaches its Taffy target, how does it get there?
    // Change three numbers here — the entire app's personality changes.
    world.default_spring(Spring {
        stiffness: 150.0, // "snappiness" — higher = faster seek
        damping:    15.0, // prevents eternal oscillation
        mass:        1.0, // affects inertia on layout transitions
    });

    // 4. RENDERING PIPELINE
    // Direct access to the wgpu pipeline config.
    world.set_renderer(RenderConfig {
        clear_color:        Color::hex("#0a0a0a"),
        msaa_samples:       4,
        vsync:              true,
        show_physics_debug: cfg!(debug_assertions), // auto-draw colliders in debug builds
    });
}
```

---

## Why Each Setting Matters

### `pixels_per_meter`

The most critical "magic number" in physics UI. Rapier operates in meters. Taffy operates in pixels. `pixels_per_meter` is the conversion constant that controls how "heavy" the UI feels relative to screen size.

| Value | Effect |
|---|---|
| `50.0` | UI feels massive, slow, weightful |
| `100.0` | Default — natural screen-scale gravity |
| `200.0` | UI feels light, snappy, almost weightless |

Expose it. Let developers feel the difference. That's the "Aha!" moment.

### Decoupled Simulation + Render

`physics_hz: 240.0` and `vsync: true` are independent settings on purpose. The physics world steps at 240Hz internally. The renderer draws at whatever the display supports — 60, 120, 144, 240Hz. Ferrum interpolates body positions between physics steps so the visual output is always smooth, never locked to the physics tick rate.

This prevents "slow-mo UI" on low-end monitors and "jittery UI" when the physics budget is tight.

### `show_physics_debug: cfg!(debug_assertions)`

Zero configuration. Run `cargo run` in debug mode and you immediately see:

- Rapier colliders (where widgets **are**)
- Taffy rects (where widgets **should be**)
- Spring forces as vectors
- Sleep state per body (green = asleep, amber = settling, red = active)

Run `cargo build --release` and it's gone. No flag, no env var.

### The Spring Default

Three numbers define the personality of the entire application:

```rust
// "Liquid UI" — slow, flowing, organic
world.default_spring(Spring { stiffness: 40.0, damping: 8.0, mass: 2.0 });

// "Standard" — natural, responsive
world.default_spring(Spring { stiffness: 150.0, damping: 15.0, mass: 1.0 });

// "Rigid UI" — fast, snappy, mechanical
world.default_spring(Spring { stiffness: 400.0, damping: 30.0, mass: 0.5 });
```

Individual components can override the default. But the default sets the baseline character — one place, whole app.

---

## The `#[component]` Macro

Components in Ferrum describe **what the widget is** and **how it behaves physically** in one block. The macro handles all the wiring.

```rust
// src/components/bouncing_button.rs
#[component]
pub fn BouncingButton(cx: Scope, label: String) -> Element {
    let is_hovered = use_signal(cx, || false);

    render! {
        RigidBody {
            // 1. PHYSICAL PROPERTIES — fluent builder
            physics: Physics::dynamic()
                .mass(2.0)
                .restitution(0.8)
                .friction(0.5),

            // 2. LAYOUT — standard Flexbox via Taffy
            style: style! {
                width:           200.px,
                height:           60.px,
                justify_content: Center,
                align_items:     Center,
                background: if *is_hovered { Color::BLUE } else { Color::RED },
            },

            // 3. EVENTS — physics-aware handlers
            on_mouseenter: move |_| is_hovered.set(true),
            on_mouseleave: move |_| is_hovered.set(false),
            on_click: move |e| {
                // Clicking physically "punches" the widget upward.
                // `e` gives direct access to the underlying Rapier body.
                e.apply_impulse(Vec2::new(0.0, -500.0));
            },

            Text { "{label}" }
        }
    }
}
```

---

## What the Macro Wires Automatically

The developer writes style + physics. The `#[component]` macro and `RigidBody` primitive handle everything else:

| Step | What happens |
|---|---|
| **Layout registration** | Node registered with Taffy. Target position computed from Flexbox/Grid rules. |
| **Body spawning** | Rapier2D rigid body + collider spawned at the Taffy target position. |
| **Constraint setup** | Spring joint created between the Taffy target and the Rapier body. Body seeks target via spring forces, not teleportation. |
| **Render binding** | `wgpu` draws an SDF quad at the body's *interpolated* transform each frame — not at the raw physics position, preventing sub-frame jitter. |
| **Sleep wiring** | Body automatically sleeps when within `0.1px` of target and velocity is near zero. Wakes on input or layout change. |

The developer never touches any of this. They write `Physics::dynamic().mass(2.0)` and the world handles it.

---

## Direct Impulse Access

The `on_click` event object `e` exposes the Rapier body directly:

```rust
on_click: move |e| {
    // Impulse — instant velocity change (a "punch")
    e.apply_impulse(Vec2::new(0.0, -500.0));

    // Force — sustained push over time (a "push")
    e.apply_force(Vec2::new(100.0, 0.0));

    // Torque — rotational impulse
    e.apply_torque_impulse(12.0);

    // Query current state
    let vel = e.body.velocity();
    let pos = e.body.position();
}
```

Clicking doesn't just trigger logic. It physically interacts with the widget. That's the difference between a UI framework and a physics UI framework.

---

## Signal Integration — No Physics Jitter

Signals update **material uniforms on the GPU**, not physics body positions. When `is_hovered` changes:

1. Signal fires
2. The `background` uniform on the widget's SDF material is updated
3. `wgpu` draws the new color on the next frame
4. The physics simulation at 240Hz is **completely unaware** this happened

Layout-affecting signal changes (width, height, flex properties) go through a separate path:

1. Signal fires
2. Taffy recomputes the target rect
3. The spring constraint updates its target position
4. The Rapier body begins seeking the new target via spring forces
5. The widget glides to its new size/position — no snap, no jitter

The physics world never resets. The spring just gets a new goal.

> **Next**: [The Signal Graph](./SIGNAL_GRAPH.md) — how data flows from `state/` into components without causing layout thrash or physics resets.
