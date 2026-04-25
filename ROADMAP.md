# Ferrum `fe_ui` — Roadmap

> **"No Hidden Magic."** If a widget is moving, the developer can always query *why* — spring force, gravity, impulse, or active transition. Physics is transparent. Always.

This roadmap takes Ferrum from experimental alpha to the framework developers *choose* for production. Four phases. Each one ships something real.

---

## Overview

| Phase | Name | Focus | Target |
|---|---|---|---|
| 1 | The Engine Room | Core stability — physics bridge, rendering, constraints | `0.2.0` |
| 2 | The Interaction Model | Gesture system, input, rubber-band, accessibility | `0.3.0` |
| 3 | Developer Experience | Hot reload, inspector, typed assets | `0.4.0` |
| 4 | Ecosystem & Distribution | Component library, WebGPU, platform shells | `1.0.0` |

---

## Phase 1 — "The Engine Room" `0.2.0`

**Goal**: Make the physics-layout bridge unbreakable. Make the renderer production-ready. This is the foundation everything else sits on.

### 1.1 — The "Settle" Logic

The single most important performance primitive in Ferrum.

- Implement an automatic **sleep state** for the physics solver
- When a widget is within `0.1px` of its Taffy target **and** velocity is below threshold → hard-snap, remove from the active step queue
- Bodies re-activate on input, layout change, or explicit wake signal
- Expose sleep/wake as observable state: `widget.physics_state()` → `Active | Settling | Asleep`

**Why it matters**: Without this, the physics world steps every frame forever. With it, a static screen costs zero physics work. Battery and CPU unlock.

```rust
// Target API
Button {
    physics: Physics::dynamic()
        .settle_threshold(0.1)   // px
        .sleep_after(3),         // frames of near-zero velocity
    "Click me"
}
```

### 1.2 — Text Excellence

Text is where UI frameworks get exposed. Ferrum has to be flawless here.

- Integrate `cosmic-text` for full **RTL**, complex shaping, and font-fallback chains
- Render glyphs via **SDF atlas** — crisp at any scale, zero re-rasterization on resize
- Glyph atlas with LRU eviction — memory-bounded, hot glyphs stay resident
- Support: emoji, CJK, Arabic, Hebrew, Devanagari out of the box
- Text layout results fed back into Taffy so wrapping text correctly influences parent dimensions

### 1.3 — The Constraint API

Developers should never configure Rapier joints directly. The `Physics` builder is the entire surface.

```rust
// Low-level (escape hatch only)
Physics::joint(RapierJoint::spring(stiffness, damping))

// High-level (what 95% of developers use)
Physics::dynamic()
    .stiffness(Stiffness::High)   // enum: Rigid | High | Medium | Low | Fluid
    .damping(0.8)                  // 0.0 = no damping, 1.0 = critically damped
    .mass(1.0)
    .restitution(0.4)
```

- `Stiffness` enum maps to pre-tuned spring coefficients — no coefficient math for the developer
- Damping `1.0` = no bounce (critically damped). `0.0` = oscillates forever
- All values hot-reloadable in Phase 3

### 1.4 — Layering & Z-Index

Physical "altitude" as a first-class concept.

- Widgets have an `altitude` property — higher altitude renders on top and casts more shadow
- Z-index computed from altitude, not declared manually (though manual override is available)
- Shadow casting: ambient shadow size/blur scales with altitude above the "ground plane"
- Collision groups: widgets at different altitudes don't interact physically unless explicitly linked

```rust
Card {
    altitude: 4.0,   // hovers. casts shadow. renders above altitude < 4.0
    shadow: Shadow::auto(), // computed from altitude
}
```

---

## Phase 2 — "The Interaction Model" `0.3.0`

**Goal**: Move beyond clicks. Build a complete sensory input system where every interaction has physical consequence.

### 2.1 — Gesture Engine

A swipe is not an event. A swipe is an impulse.

- `touchstart` / `touchmove` / `touchend` → velocity tracking → impulse injection into Rapier body
- Gesture recognizer sits *before* the physics layer — gesture intent is resolved, then physics handles the result
- Built-in gestures: `Swipe`, `Pinch`, `LongPress`, `Drag`, `Flick`
- Drag: widget follows touch position via a spring motor, not a position set — it *lags* correctly
- Flick: releases the spring motor and injects the terminal velocity as an impulse

```rust
ScrollArea {
    gesture: Gesture::flick()
        .friction(0.92)       // per-frame velocity decay
        .boundary(Edge::rubber_band(0.3)), // rubber-band stiffness
}
```

### 2.2 — Focus & Keyboard Navigation

Spatial navigation with physical character.

- `Tab` / `Shift+Tab` moves focus through the logical tree
- Focus highlight is a **physics body** — it slides from the previous focused element to the next via a spring, not a CSS transition
- Arrow key navigation for spatial layouts: moves focus to the nearest widget in that direction
- All keyboard navigation respects `AccessKit` node ordering

```rust
// Focus ring is a real widget with physics
FocusRing {
    stiffness: Stiffness::High,
    damping: 0.75,
    color: theme.accent,
}
```

### 2.3 — Rubber Band Edge

Over-scroll that feels native because it *is* native physics.

- `ScrollArea` boundary is a soft constraint, not a hard clamp
- Scrolling past the edge stretches against a configurable spring — resistance increases with displacement
- Release → spring pulls back with correct velocity and settles
- No fake easing curves. Rapier handles all of it.

```rust
ScrollArea {
    overscroll: Overscroll::rubber_band()
        .stiffness(180.0)
        .max_displacement(80.0), // px before hard stop
}
```

### 2.4 — AccessKit Integration

Accessibility is not optional. It ships in Phase 2, not Phase 4.

- Semantic tree mirrors the physics world — node positions update as bodies move
- Screen readers receive position updates on every physics settle, not just on render
- `aria-live` equivalent for widgets in motion: announces movement intent, not mid-flight position
- All `fe_std` components (Phase 4) ship with AccessKit roles pre-configured

---

## Phase 3 — "Developer Experience" `0.4.0`

**Goal**: Make building with Ferrum feel like magic. Zero-friction iteration. Full observability.

### 3.1 — Hot Reloading

The holy grail. Two independent hot-reload systems.

**Component Logic** (`.dylib` / `.so` swap)
- Components compile to a dynamic library
- File watcher detects changes → recompile → swap the dylib handle
- wgpu context, physics world, and widget tree survive the swap
- Widget *state* is preserved across reloads via a serialization boundary

**Shaders + Physics Coefficients** (live config)
- `.wgsl` shaders hot-reload without touching the application
- Physics constants (gravity, spring coefficients, damping) live in a config file that reloads instantly
- Inspector (see 3.2) reflects changes in real time

```toml
# ferrum.toml — hot-reloaded on save
[physics]
gravity = [0.0, 980.0]
default_damping = 0.75

[theme]
accent = "#7c3aed"
```

### 3.2 — The Ferrum Inspector

Triggered by `F12` in debug builds. Zero overhead in release.

Three overlays, toggled independently:

| Overlay | Shows |
|---|---|
| **Layout** | Taffy-computed rects — where widgets *should* be |
| **Physics** | Rapier colliders, velocity vectors, sleep state, altitude — where widgets *are* |
| **Signals** | Live signal graph — data flow, stale nodes, update count per frame |

Additional inspector features:
- Click any widget → inspect its full physics state, constraint config, and signal subscriptions
- Time scrubber: record 5 seconds of physics state and replay frame-by-frame
- Export physics log as JSON for offline debugging

### 3.3 — Typed Assets

Assets that fail at compile time, not at runtime.

```rust
// Macro generates typed handles at compile time
// Missing file = compile error, not a runtime panic
assets! {
    fonts: {
        mono: "assets/fonts/JetBrainsMono.ttf",
        display: "assets/fonts/Satoshi.ttf",
    },
    shaders: {
        blur: "assets/shaders/blur.wgsl",
    },
    images: {
        logo: "assets/images/logo.png",
    }
}

// Usage — fully typed, no string lookups
Text { font: Assets::fonts::mono, "Hello" }
```

- Asset pipeline runs at build time via a `build.rs` proc-macro
- Generates strongly-typed handles into a sealed `Assets` module
- Hot-reload in debug, embedded binary in release

---

## Phase 4 — "Ecosystem & Distribution" `1.0.0`

**Goal**: Ship to real screens. Build the primitives developers need. Stabilize the API.

### 4.1 — `fe_std` Component Library

Physics-tuned primitives. Every component ships with sensible defaults and full customization.

| Component | Physics Character |
|---|---|
| `ScrollArea` | Flick momentum + rubber-band edges |
| `Modal` | Drops from top with gravity, catches on a spring at center |
| `Slider` | Thumb has mass, snaps to discrete values with small impulse |
| `Switch` | Toggle snaps with restitution — satisfying click-feel |
| `Toast` | Slides in with momentum, auto-dismisses with gravity |
| `Drawer` | Drag to open, momentum-aware release (open or close based on velocity) |
| `Button` | Press deforms (scale spring), release bounces |

All components:
- Fully accessible (AccessKit roles + keyboard navigation)
- Themeable via CSS-like property inheritance
- Physics properties overridable at usage site

### 4.2 — WebGPU Target

Same codebase, browser-native performance.

- Stabilize the `wasm32-unknown-unknown` + WebGPU pipeline
- Physics runs on the same Rapier WASM build — no separate browser physics
- Bundle size target: `< 2MB` gzipped for a minimal app
- `fe_ui_web` crate handles browser-specific windowing (canvas, resize events, input)

### 4.3 — Platform Shells

Native character on every platform.

| Platform | Native Integration |
|---|---|
| **macOS** | Title bar transparency, vibrancy materials, native menu bar |
| **Windows** | Acrylic/Mica window background, snap layout awareness |
| **Linux** | GTK/libadwaita theme detection, portal integration for file dialogs |
| **iOS** | Safe area insets, haptic feedback on physics settle/impact |
| **Android** | Edge-to-edge display, predictive back gesture integration |

---

## Stability Commitment

| Version | API Stability |
|---|---|
| `0.x.x` | No stability guarantees. Documented breaking changes in `CHANGELOG.md` |
| `1.0.0` | Stable public API. Semver respected. Deprecation before removal |
| `fe_ui_core` | Stabilizes first — core types, signals, component model |
| `fe_std` | Stabilizes with `1.0.0` |
| `fe_ui_rapier` / `fe_ui_wgpu` | Internal crates — not public API |

---

## The Ferrum Philosophy

These are not guidelines. They are constraints. Breaking them requires a discussion.

1. **Feel > Features** — 60fps minimum. `<8ms` input latency is the goal. A sluggish Ferrum app ships nothing.
2. **Physics is UX** — Motion communicates. iOS didn't add springs for aesthetics. Every animation in `fe_std` has a physical reason.
3. **Native or Nothing** — No webviews. No JS runtime. If it can't run on a Raspberry Pi, it doesn't ship.
4. **No Hidden Magic** — Any widget in motion exposes *why* it is moving. `widget.force_debug()` is always available in debug builds.
5. **Compile Times Matter** — `fe_ui_core` compiles in `<3s`. The hot path stays macro-free. Proc-macros are for assets and components only.

---

## Contributing

Check `CONTRIBUTING.md` for setup. The highest-value contributions right now:

- **Phase 1**: The settle/sleep logic — `fe_ui_rapier/src/solver.rs`
- **Phase 1**: SDF glyph atlas — `fe_ui_wgpu/src/text/atlas.rs`
- **Phase 2**: Gesture velocity tracking — `fe_ui_core/src/input/gesture.rs`

Join Discord: [link] — we argue about spring coefficients and damping ratios.

---

**Ferrum** = Latin for iron. **`fe_ui`** = ironclad UI.

Built different.
