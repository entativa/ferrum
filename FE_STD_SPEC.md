# Ferrum `fe_std` — Standard Component Library Spec

> **"Predictable Controls, Alive Containers."**
>
> Precision components are kinematic — spring-driven, pixel-perfect, trustworthy.
> Spatial components are dynamic — impulse-driven, gravity-aware, alive.

---

## Foundational Decisions

Before the components, three framework-level contracts that every `fe_std` primitive honors.

### 1. The Parent-Joint Model

Children are isolated physics bodies, but they are **fixed-jointed to their parent by default**. When a `Modal` drops with gravity, its children move with it — not because they share a body, but because they are physically welded to the container.

The weld upgrades automatically when a child declares its own physics:

```rust
// Default — fixed joint (weld). Child moves exactly with parent.
Text { "I move with the Modal" }

// Opt-in — spring joint. Child follows parent with physical lag/jiggle.
Avatar {
    physics: Physics::dynamic()
        .stiffness(Stiffness::Medium)
        .damping(0.6),
    // Lags behind on fast moves. Bounces on settle. Feels alive.
}
```

| Joint Type | Trigger | Behavior |
|---|---|---|
| **Fixed (Weld)** | Default — no `physics` prop | Child is rigidly attached. Moves identically to parent. |
| **Spring** | Child declares `Physics::dynamic()` | Child follows parent via spring force. Lags, jiggles, settles. |
| **Free** | Child declares `Physics::free()` | Child detaches on spawn. Governed only by its own forces. |

### 2. The Theming Contract — SDF Uniform Tier

Standard components expose a `Theme` struct that maps directly to SDF uniforms. Everything stays Tier 0 — one draw call for the entire `fe_std` library.

```rust
// What every fe_std component accepts
pub struct ComponentTheme {
    pub color:         Color,      // fill — maps to SDF color uniform
    pub color_hovered: Color,      // fill on hover
    pub color_pressed: Color,      // fill on press
    pub border_color:  Color,      // maps to SDF border_color uniform
    pub border_width:  f32,        // maps to SDF border_width uniform
    pub corner_radius: f32,        // maps to SDF corner_radius uniform
    pub glow_color:    Color,      // maps to SDF glow_color uniform
    pub glow_radius:   f32,        // maps to SDF glow_radius uniform
    pub altitude:      f32,        // shadow depth — derived in shader
}
```

**The rule**: If it's in `ComponentTheme`, it's Tier 0. If it needs a custom shader, use `CustomBox`.

```rust
// The escape hatch — explicit, honest about the cost
CustomBox {
    shader: "assets/shaders/my_effect.wgsl",
    // ⚠ This breaks the SDF batch. One extra draw call per unique shader.
    // Use sparingly. Prefer ComponentTheme for standard customization.
}
```

### 3. The Animation Contract — Interaction Classes

| Class | Components | Mechanism | Character |
|---|---|---|---|
| **Kinematic-Spring** | Switch, Slider, Tabs, Checkbox | Spring to a layout target | Pixel-perfect, predictable, trustworthy |
| **Dynamic-Impulse** | Modal, Toast, Drawer, List items | Gravity + impulse | Alive, flick-able, physical |

Kinematic components never fight the user. Dynamic components reward the user's input velocity.

---

## Component Specifications

---

### `Button`

**Class**: Kinematic-Spring (press deformation) + Dynamic-Impulse (on explicit throw)

The most fundamental component. Press deforms it via a scale spring. Click applies an upward impulse. Release lets it settle back.

```rust
Button {
    // Physics — kinematic by default, dynamic on click
    physics: Physics::kinematic()
        .press_deform(0.92)      // scale on press (spring-driven)
        .restitution(0.6),       // bounce on release

    // Theme — Tier 0 only
    theme: ButtonTheme {
        color:         Color::hex("#7c3aed"),
        color_hovered: Color::hex("#6d28d9"),
        color_pressed: Color::hex("#5b21b6"),
        border_width:  1.5,
        border_color:  Color::hex("#a78bfa"),
        corner_radius: 10.0,
        glow_color:    Color::hex("#7c3aed"),
        glow_radius:   0.0,       // 0.0 = no glow at rest
        altitude:      2.0,
    },

    // Interaction
    on_click: move |e| {
        e.apply_impulse(Vec2::new(0.0, -300.0)); // physical jump on click
    },

    "Click me"
}
```

**Physics lifecycle**:
1. At rest → kinematic body at Taffy target, sleep state
2. Hover → wake, glow_radius spring to `8.0`, altitude spring to `3.0`
3. Press → scale spring to `0.92`, altitude spring to `1.0`
4. Release → impulse applied, scale spring back to `1.0`, altitude spring to `2.0`, body settles

---

### `Switch`

**Class**: Kinematic-Spring (Precision Control)

A Switch is a control surface. The user must trust it. It must arrive at its target position every time — no overshoot, no ambiguity.

```rust
Switch {
    value: is_enabled,
    on_change: move |val| is_enabled.set(val),

    physics: Physics::kinematic()
        .stiffness(Stiffness::High)  // fast seek
        .damping(1.0),               // critically damped — zero overshoot

    theme: SwitchTheme {
        track_color_off: Color::hex("#374151"),
        track_color_on:  Color::hex("#7c3aed"),
        thumb_color:     Color::hex("#ffffff"),
        corner_radius:   999.0,  // pill shape
        altitude:        1.0,
    },
}
```

**Why critically damped (`damping: 1.0`)**: The thumb reaches its target in minimum time with zero overshoot. It never bounces past the ON/OFF position. The user always knows the state. This is the difference between a control and a toy.

**Physics lifecycle**:
1. Toggle → `use_layout_signal` target flips (left anchor ↔ right anchor)
2. Sync Point → Taffy resolves new thumb target rect
3. Spring seeks new target — critically damped, arrives clean
4. Track color → `use_signal` crossfade via GPU uniform (no layout involvement)

---

### `Slider`

**Class**: Kinematic-Spring (Precision Control)

The thumb has mass. Dragging it feels like pulling something with weight. Releasing it snaps to the nearest discrete value (if stepped) with a small satisfying impulse.

```rust
Slider {
    value: volume,
    range: 0.0..=1.0,
    step: Some(0.05),   // None = continuous

    physics: Physics::kinematic()
        .mass(0.8)               // thumb feels weighted during drag
        .stiffness(Stiffness::High)
        .damping(0.85),          // slight overshoot on snap — intentional

    on_change: move |val| volume.set(val),

    theme: SliderTheme {
        track_color:  Color::hex("#1f2937"),
        fill_color:   Color::hex("#7c3aed"),
        thumb_color:  Color::hex("#ffffff"),
        thumb_radius: 10.0,
        altitude:     1.5,
    },
}
```

**The snap impulse**: When the user releases the thumb mid-step, a small impulse (`Vec2::new(direction * 40.0, 0.0)`) is applied before the spring seeks the snap target. The thumb travels to the step position with visible momentum — not a teleport, not a float. A *snap*.

---

### `ScrollArea`

**Class**: Dynamic-Impulse (Spatial Container) with Kinematic rubber-band

The content inside a `ScrollArea` is a dynamic body. Flicking applies a real velocity impulse. The edges are spring colliders (rubber-band). The thumb is kinematic.

```rust
ScrollArea {
    direction: ScrollDirection::Vertical,

    physics: ScrollPhysics {
        friction:         0.88,   // per-frame velocity decay (1.0 = no friction)
        overscroll: Overscroll::RubberBand {
            stiffness:        180.0,
            max_displacement:  80.0,  // px before hard stop
        },
    },

    portal: Portal::bounded()
        .collider(true),

    theme: ScrollTheme {
        thumb_color:        Color::hex("#374151"),
        thumb_color_active: Color::hex("#6b7280"),
        thumb_width:        4.0,
        corner_radius:      999.0,
    },

    // Children — fixed-jointed to the scroll body by default
    PlayerCard { }
    PlayerCard { }
    PlayerCard { }
}
```

**Flick lifecycle**:
1. `touchstart` / `mousedown` → gesture engine begins velocity tracking
2. `touchmove` → scroll body follows via spring motor (lags correctly)
3. `touchend` → spring motor released, terminal velocity injected as impulse
4. Body decelerates via friction (`0.88` per frame)
5. If body reaches portal boundary → rubber-band spring resists, then pulls back
6. Body velocity drops below sleep threshold → sleep state, zero CPU cost

---

### `Modal`

**Class**: Dynamic-Impulse (Spatial Component)

A Modal is a physical object that enters and exits the scene. It drops from above the viewport with gravity. It catches on a spring at center. It can be flicked to dismiss.

```rust
Modal {
    visible: show_modal,

    // Entry — spawned above viewport, falls with gravity
    entry: ModalEntry::DropFromTop {
        spawn_y_offset: -200.0,   // px above viewport top
        gravity_scale:  1.2,      // slightly exaggerated gravity
    },

    // Exit — flick to dismiss, or programmatic
    dismiss: ModalDismiss::Flick {
        velocity_threshold: 600.0,  // px/s — below this, snaps back
        exit_gravity_scale: 2.0,    // accelerates away on dismiss
    },

    // Rest position — spring target at center
    physics: Physics::dynamic()
        .stiffness(Stiffness::Medium)
        .damping(0.75)
        .altitude(8.0),   // high altitude — deep shadow, renders over everything

    theme: ModalTheme {
        color:         Color::hex("#0d1117"),
        border_color:  Color::hex("#21262d"),
        border_width:  1.0,
        corner_radius: 16.0,
        altitude:      8.0,
    },

    // Children — fixed-jointed by default, jiggle with Physics::dynamic()
    ModalHeader { "Settings" }
    ModalBody { /* content */ }
}
```

**Entry lifecycle**:
1. `visible` signal → `true`
2. Modal body spawned at `spawn_y_offset` above viewport (outside Portal)
3. Gravity pulls it downward
4. Catches spring at center — `stiffness: Medium`, `damping: 0.75` → small overshoot, settles
5. Children (fixed-jointed) arrive with it. Any `Physics::dynamic()` child jiggles on settle.

**Dismiss lifecycle**:
1. User flick exceeds `velocity_threshold`
2. Spring constraint released
3. `exit_gravity_scale` applied — accelerates away
4. Body exits viewport → despawned after 500ms buffer
5. `visible` signal → `false` queued for next frame Sync Point

---

### `Toast`

**Class**: Dynamic-Impulse (Spatial Component)

A notification that slides in from the top edge with momentum, hovers, then exits with gravity on timeout or swipe.

```rust
Toast {
    message: "Track saved.",
    duration: Duration::secs(3),

    entry: ToastEntry::SlideFromTop {
        initial_velocity: Vec2::new(0.0, 400.0), // downward entry velocity
    },

    exit: ToastExit::FallDown {  // or FallUp, SlideRight, Flick
        gravity_scale: 1.5,
    },

    physics: Physics::dynamic()
        .altitude(6.0)
        .stiffness(Stiffness::High)
        .damping(0.8),

    theme: ToastTheme {
        color:         Color::hex("#111827"),
        border_color:  Color::hex("#374151"),
        border_width:  1.0,
        corner_radius: 12.0,
        altitude:      6.0,
    },
}
```

---

### `Drawer`

**Class**: Dynamic-Impulse (Spatial Component)

A panel that slides in from an edge. Drag to open, momentum-aware release — if released above half-open with enough upward velocity, it opens. Below, it closes.

```rust
Drawer {
    edge:    DrawerEdge::Left,
    visible: show_drawer,

    physics: DrawerPhysics {
        drag_stiffness:   200.0,   // resistance during drag (spring motor)
        open_threshold:   0.4,     // fraction open required to auto-complete
        velocity_threshold: 300.0, // px/s — overrides position threshold
    },

    theme: DrawerTheme {
        color:         Color::hex("#0d1117"),
        border_color:  Color::hex("#21262d"),
        border_width:  1.0,
        altitude:      6.0,
        corner_radius: 0.0,  // full-edge drawers have no rounding
    },

    // Scrim — separate kinematic body, alpha tied to drawer open fraction
    scrim: Scrim {
        color: Color::rgba(0.0, 0.0, 0.0, 0.6),
        on_tap: move |_| show_drawer.set(false),
    },

    DrawerContent { }
}
```

**Release decision logic** (runs at gesture end):
```
if open_fraction > open_threshold OR velocity > velocity_threshold (opening direction):
    → apply impulse toward fully open, spring seeks open target
else:
    → apply impulse toward fully closed, spring seeks closed target
```

The physics decide. The developer just sets the thresholds.

---

### `TabBar`

**Class**: Kinematic-Spring (Precision Control)

The active indicator is a physical body that slides between tab positions. Fast switches leave the indicator traveling — it arrives after the label change.

```rust
TabBar {
    selected: active_tab,
    on_change: move |tab| active_tab.set(tab),

    indicator_physics: Physics::kinematic()
        .stiffness(Stiffness::High)
        .damping(0.75),   // slight overshoot — indicator "lands" on the tab

    tabs: vec![
        Tab { id: 0, label: "Home"    },
        Tab { id: 1, label: "Search"  },
        Tab { id: 2, label: "Library" },
    ],

    theme: TabTheme {
        indicator_color:  Color::hex("#7c3aed"),
        label_color:      Color::hex("#6b7280"),
        label_color_active: Color::hex("#f9fafb"),
        corner_radius:    4.0,
        altitude:         0.5,
    },
}
```

The indicator travels to the new tab's position via spring. During travel, it slightly stretches horizontally (scale spring on X axis, `restitution: 0.3`) — like a liquid blob moving between targets. Arrives, overshoots slightly, settles.

---

## Component Summary

| Component | Class | Joint Default | Batch Tier | Notes |
|---|---|---|---|---|
| `Button` | Kinematic-Spring | Fixed | Tier 0 | Dynamic impulse on click |
| `Switch` | Kinematic-Spring | Fixed | Tier 0 | Critically damped — zero overshoot |
| `Slider` | Kinematic-Spring | Fixed | Tier 0 | Snap impulse on step release |
| `ScrollArea` | Dynamic-Impulse | Fixed | Tier 0 | Rubber-band via spring collider |
| `Modal` | Dynamic-Impulse | Fixed → Spring (opt-in) | Tier 0 | Children jiggle with `Physics::dynamic()` |
| `Toast` | Dynamic-Impulse | Fixed | Tier 0 | Configurable entry/exit vectors |
| `Drawer` | Dynamic-Impulse | Fixed | Tier 0 | Velocity-aware open/close decision |
| `TabBar` | Kinematic-Spring | Fixed | Tier 0 | Indicator stretches during travel |

**All Tier 0. One draw call. Combined.**

---

## The fe_std Physics Budget

Every component is designed to cost zero physics work when at rest:

- Kinematic components → sleep immediately on settle
- Dynamic containers → sleep when all child bodies are settled
- ScrollArea → sleep when scroll velocity drops below threshold
- Modal/Toast/Drawer → despawned after exit animation completes (no sleeping body left in world)

A fully static screen with all `fe_std` components at rest: **zero Rapier substeps**. The physics world sleeps entirely until the next input event.

---

## What `fe_std` Is Not

- **Not a design system**: `fe_std` ships unstyled defaults. Bring your own `Theme`.
- **Not opinionated about layout**: Components accept standard Taffy flex/grid properties. Compose them however you want.
- **Not exhaustive**: `fe_std` covers the primitives. Complex patterns (virtualized lists, date pickers, rich text editors) are community crates built on top.

---

*See also: [Rendering Pipeline](./RENDERING_PIPELINE.md) — how every Tier 0 component hits the GPU.*
*See also: [Signal Graph](./SIGNAL_GRAPH.md) — how `use_layout_signal` drives kinematic spring targets.*
*See also: [Scene Spec](./SCENE_SPEC.md) — the global spring defaults that fe_std inherits.*
