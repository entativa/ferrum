# Ferrum `fe_ui` — Project Structure

> Separate the "boring" application logic from the "living" visual world. Avoid the massive `main.rs` trap.

In Ferrum, you aren't *rendering a view* — you're *staging a scene*. The project layout reflects that distinction at every level.

---

## Standard Project Layout

```
my_ferrum_app/
├── assets/                     # Non-code resources
│   ├── fonts/                  # .ttf / .otf handled by cosmic-text
│   ├── shaders/                # Custom .wgsl SDF effects and materials
│   └── images/                 # Textures for the GPU atlas
│
├── src/
│   ├── components/             # Reusable UI atoms — each one a physical entity
│   │   ├── mod.rs
│   │   ├── sidebar.rs
│   │   └── player_card.rs      # Components define their own physics behavior
│   │
│   ├── state/                  # Global signals and business logic
│   │   └── store.rs            # Signals, derived state, actions
│   │
│   ├── main.rs                 # App entry point & window config
│   └── scene.rs                # Root render tree + physics world setup
│
├── .ferrum/                    # Tooling-specific (generated, gitignore this)
│   └── cache/                  # Pre-compiled SDF glyphs and shaders
│
├── build.rs                    # Asset bundling and compile-time shader validation
└── Cargo.toml
```

---

## Why This Layout Works

### `src/scene.rs` — The World View

`scene.rs` is the coordinator between Taffy (the grid) and Rapier (the gravity). It's where the physics world is configured and the root widget tree is declared.

If you want to change the global gravity, the rubbery feel of all windows, or the default damping across the app — you change it here. One file. One source of truth.

```rust
// src/scene.rs
pub fn scene(cx: Scope) -> Element {
    render! {
        PhysicsWorld {
            gravity: Vec2::new(0.0, 980.0),
            default_damping: 0.75,
            default_stiffness: Stiffness::Medium,

            // Your root layout lives here
            Flex {
                direction: Column,
                Sidebar {}
                MainContent {}
            }
        }
    }
}
```

### `src/components/` — Physical Entities

Each component owns its physics behavior. The physics config lives *with* the component, not in a global style sheet or a separate config file. When you open `player_card.rs`, you see exactly how it moves.

```rust
// src/components/player_card.rs
#[component]
pub fn PlayerCard(cx: Scope) -> Element {
    render! {
        Box {
            physics: Physics::dynamic()
                .restitution(0.5)   // bounces when it hits the bottom of the list
                .drag(2.0),         // feels "heavy" to drag
            style: style! {
                width: 300.px,
                height: 150.px,
                background: Color::hex("#2d2d2d"),
            },
            Text { "Player One" }
        }
    }
}
```

### `src/state/store.rs` — Signals, Not State Machines

Business logic lives completely outside the render tree. No component reaches into another component's internals — everything flows through signals declared in `store.rs`.

```rust
// src/state/store.rs
pub struct AppStore {
    pub track_title: Signal<String>,
    pub is_playing: Signal<bool>,
    pub volume: Signal<f32>,
}

impl AppStore {
    pub fn new(cx: Scope) -> Self {
        Self {
            track_title: use_signal(cx, || "Untitled".into()),
            is_playing: use_signal(cx, || false),
            volume: use_signal(cx, || 1.0),
        }
    }
}
```

### `assets/shaders/` — Custom Materials

Since Ferrum is built on `wgpu` and SDF rendering, developers can write custom `.wgsl` materials for any widget — a button that glows on hover, a card that ripples on press, a blur effect that responds to altitude.

Ferrum's file watcher hot-reloads `.wgsl` files in debug builds. Change your blur radius or your glow color and see it update in real time, no recompile.

```wgsl
// assets/shaders/glow_button.wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sdf = rounded_rect_sdf(in.uv, in.size, in.radius);
    let glow = smoothstep(0.0, 0.02, -sdf) * in.glow_intensity;
    return mix(in.base_color, in.glow_color, glow);
}
```

### `build.rs` — The Secret Sauce

`build.rs` does the expensive work at compile time so the runtime doesn't have to.

- **Shader validation**: `.wgsl` files are parsed and type-checked via `naga` before the binary is produced. Invalid shader = compile error, not a runtime crash.
- **Asset bundling**: Fonts, images, and shaders are processed into optimized formats and embedded or staged for the GPU atlas.
- **Typed asset generation**: The `assets!` macro output is generated here — strongly typed handles for every asset file, checked at compile time.

```rust
// build.rs
fn main() {
    ferrum_build::AssetPipeline::new()
        .validate_shaders("assets/shaders/")
        .bundle_fonts("assets/fonts/")
        .stage_images("assets/images/")
        .emit_typed_handles() // generates src/assets.rs
        .run();
}
```

---

## The `.ferrum/` Directory

Generated by the toolchain. Never edit manually. Add to `.gitignore`.

```
.ferrum/
└── cache/
    ├── glyphs/        # Pre-rasterized SDF glyph atlas
    ├── shaders/       # Compiled SPIR-V / MSL / DXIL cache
    └── assets.ron     # Asset manifest for the hot-reload watcher
```

The cache means cold-start time drops significantly after the first build — glyphs and shaders don't re-process unless the source files change.

---

## `main.rs` — Entry Point Only

`main.rs` does one thing: configure the window and hand off to `scene.rs`. Nothing else lives here.

```rust
// src/main.rs
use fe_ui::prelude::*;
mod components;
mod state;
mod scene;

fn main() {
    App::new()
        .add_window(
            Window::default()
                .title("My Ferrum App")
                .size(1280, 800)
                .theme(Theme::dark()),
        )
        .run(scene::scene);
}
```

---

## Gitignore

```gitignore
# Ferrum generated files
.ferrum/

# Standard Rust
/target
Cargo.lock   # remove this line if building a binary, keep for libraries
```
