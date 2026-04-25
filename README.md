# Ferrum `fe_ui`

**GPU-accelerated Rust UI framework with physics-native layout.**

`fe_ui` treats your interface like a game world. Layout via Taffy, rendering via wgpu, physics via Rapier2D. Every widget has mass, every interaction has momentum. Down to the metal, everywhere.

[![Crates.io](https://img.shields.io/crates/v/fe_ui.svg)](https://crates.io/crates/fe_ui)
[![Docs](https://docs.rs/fe_ui/badge.svg)](https://docs.rs/fe_ui)
[![License](https://img.shields.io/crates/l/fe_ui.svg)](./LICENSE)

> **Status: Experimental** — We're in early alpha. APIs will break. Physics will be janky. That's the fun part.

---

### Why Ferrum?

Modern UI feels dead. Clicks pop. Scrolls slide. Ferrum makes it *alive*.

|  | egui | Iced | Dioxus | Flutter | **fe\_ui** |
|---|---|---|---|---|---|
| GPU Rendering | ✅ | ✅ | ✅ | ✅ | **✅ wgpu** |
| Layout | Immed | Elm | Taffy | Custom | **Taffy** |
| Physics | ❌ | ❌ | ❌ | ❌ | **✅ Rapier2D** |
| Feel | Tool | App | App | App | **Game** |

**Core Idea**: Taffy decides where things *should* be. Rapier decides where things *are*. A constraint solver makes them agree via springs. Result: UI that bounces, collides, and settles instead of teleporting.

---

### Features

- **Down to the Metal**: `wgpu` backend → Vulkan, Metal, DX12, WebGPU. One codebase, all platforms.
- **Layout That Doesn't Suck**: Full Flexbox + Grid + Block via `taffy`. CSS-level power, Rust-level speed.
- **Physics by Default**: `Rapier2D` built-in. Buttons have mass. Lists have friction. Scroll has real inertia.
- **Constraint Solver**: Layout goals become soft spring joints. Resize a window and watch widgets glide to position instead of snapping.
- **Zero-cost Reactivity**: Fine-grained signals. No VDOM. Update 1 text node = 1 draw call, not a tree diff.
- **SDF Rendering**: Text, shapes, shadows = signed distance fields. Crisp at 8K, 60fps on mobile.

---

### Quick Start

```toml
[dependencies]
fe_ui = "0.1.0-alpha"
```

```rust
use fe_ui::prelude::*;

fn main() {
    App::new()
        .add_window(Window::default())
        .run(ui);
}

#[component]
fn ui(cx: Scope) -> Element {
    let count = use_signal(cx, || 0);

    render! {
        Flex {
            direction: Column,
            gap: 16.0,
            padding: 32.0,
            physics_world: PhysicsWorld::new(Vec2::new(0.0, 900.0)), // gravity

            Text { size: 32.0, "Count: {count}" }

            Button {
                onclick: move |_| count += 1,
                physics: Physics::dynamic()
                    .mass(1.0)
                    .restitution(0.7), // bouncy
                "Click me"
            }
        }
    }
}
```

Click the button. It physically jumps. That's Ferrum.

---

### How It Works

1. **Layout**: `taffy` computes target positions from Flexbox/Grid.
2. **Constraints**: Targets become spring/motors in `rapier2d`.
3. **Physics**: Step the world at 240Hz. Bodies move toward targets with forces.
4. **Render**: `wgpu` draws SDF quads at interpolated body positions.

No fighting. No snapping. Layout and physics cooperate through the constraint solver.

---

### Project Structure

```
fe_ui/
├── crates/
│   ├── fe_ui_core/     # Core types, signals, components. No GPU.
│   ├── fe_ui_taffy/    # Taffy integration + layout caching
│   ├── fe_ui_rapier/   # Rapier integration + constraint solver
│   ├── fe_ui_wgpu/     # wgpu renderer, SDF materials, atlas
│   └── fe_ui/          # Main crate, prelude, macros
├── examples/           # `cargo run --example bouncing_button`
└── README.md
```

---

### Platform Support

| Platform | Status | Backend |
|---|---|---|
| Windows | ✅ | DX12 |
| macOS | ✅ | Metal |
| Linux | ✅ | Vulkan |
| Web | 🚧 | WebGPU/Wasm |
| iOS | 📝 Planned | Metal |
| Android | 📝 Planned | Vulkan |

Windowing via `winit`. Accessibility via `accesskit`.

---

### Roadmap

**0.1.0 — "Hello Bounce"**
- [x] winit + wgpu window
- [x] Taffy layout → positioned quads
- [ ] Rapier constraint solver MVP
- [ ] SDF rounded rect + text via `cosmic-text`

**0.2.0 — "Actually Usable"**
- [ ] Input + focus system
- [ ] Scroll with physics momentum
- [ ] Hot reload wgsl + view code
- [ ] DevTools: physics debugger

**1.0.0 — "Game-feel UI"**
- [ ] Stable API
- [ ] Mobile targets
- [ ] Editor / visual layout tool

---

### Contributing

This is early days. The best way to contribute is to break things.

1. Check [Issues](https://github.com/ferrum-ui/fe_ui/issues) for `good-first-physics-bug`
2. Read `ARCHITECTURE.md` to understand the constraint solver
3. Join Discord: [link] — we argue about spring coefficients

---

### Philosophy

1. **Feel > Features**: 60fps is the minimum. <8ms input latency is the goal.
2. **Physics is UX**: iOS didn't add springs for fun. Motion communicates.
3. **Native or Nothing**: No webviews. No JS. If it can't run on a Raspberry Pi, it doesn't ship.
4. **Compile Times Matter**: `fe_ui_core` compiles in <3s. Keep the hot path macro-free.

---

### License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

**Ferrum** = Latin for iron. **fe\_ui** = ironclad UI.

Built different.
