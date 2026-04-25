#!/usr/bin/env python3
"""
scaffold_ferrum.py — Ferrum `fe_ui` Project Scaffolder
=======================================================
Generates the complete Ferrum framework directory tree alongside
an existing README.md. Run this script from the directory that
already contains your README.md.

Usage:
    python scaffold_ferrum.py [--root PATH]

    --root PATH   Directory to scaffold into (default: current directory)
                  Must already contain README.md.
"""

import argparse
import os
import sys
import textwrap
from pathlib import Path

# ──────────────────────────────────────────────────────────────────────────────
# ANSI colours
# ──────────────────────────────────────────────────────────────────────────────
GREEN  = "\033[92m"
YELLOW = "\033[93m"
CYAN   = "\033[96m"
BOLD   = "\033[1m"
RESET  = "\033[0m"

def info(msg: str)  -> None: print(f"{CYAN}  {msg}{RESET}")
def ok(msg: str)    -> None: print(f"{GREEN}  ✓ {msg}{RESET}")
def warn(msg: str)  -> None: print(f"{YELLOW}  ⚠ {msg}{RESET}")
def header(msg: str)-> None: print(f"\n{BOLD}{msg}{RESET}")


# ──────────────────────────────────────────────────────────────────────────────
# FILE CONTENTS
# Each entry is (relative_path, content_string).
# Empty stubs get a minimal comment header so the file isn't truly empty.
# ──────────────────────────────────────────────────────────────────────────────

def stub(path: str, description: str = "") -> tuple[str, str]:
    """Return a (path, content) tuple for an empty stub file."""
    ext = Path(path).suffix
    comment = {
        ".rs":   f"// {description or path}\n// TODO: implement\n",
        ".toml": f"# {description or path}\n",
        ".wgsl": f"// {description or path}\n// TODO: implement shader\n",
        ".md":   f"# {Path(path).stem.replace('_', ' ').title()}\n\n> TODO\n",
        ".ron":  f"// {description or path}\n",
        ".gitignore": "",
        ".gitkeep": "",
    }.get(ext, f"// {description or path}\n")
    return (path, comment)


FILES: list[tuple[str, str]] = [

    # ──────────────────────────────────────────────────────────────────────
    # Root workspace
    # ──────────────────────────────────────────────────────────────────────
    ("Cargo.toml", textwrap.dedent("""\
        [workspace]
        resolver = "2"
        members = [
            "crates/fe_ui_core",
            "crates/fe_ui_taffy",
            "crates/fe_ui_rapier",
            "crates/fe_ui_wgpu",
            "crates/fe_ui",
            "crates/fe_std",
        ]

        [workspace.dependencies]
        # Core
        log        = "0.4"
        thiserror  = "1"
        slotmap    = "1"

        # Layout
        taffy = "0.4"

        # Physics
        rapier2d = { version = "0.18", features = ["enhanced-determinism"] }

        # Rendering
        wgpu     = "0.20"
        winit    = "0.30"
        bytemuck = { version = "1", features = ["derive"] }

        # Text
        cosmic-text = "0.11"

        # Math
        glam = "0.27"

        # Accessibility
        accesskit = "0.16"

        [profile.dev]
        opt-level = 1          # faster incremental builds

        [profile.release]
        lto       = "thin"
        opt-level = 3
        strip     = true
    """)),

    ("ferrum.toml", textwrap.dedent("""\
        # ferrum.toml — hot-reloaded in debug builds
        # Change these values while the app is running to tweak physics live.

        [physics]
        gravity         = [0.0, 980.0]   # pixels/s²  (100 px = 1 m)
        air_density     = 0.1
        pixels_per_meter = 100.0

        [simulation]
        physics_hz        = 240.0
        substeps          = 4
        solver_iterations = 8

        [spring]
        stiffness = 150.0
        damping   = 15.0
        mass      = 1.0

        [renderer]
        clear_color  = "#0a0a0a"
        msaa_samples = 4
        vsync        = true

        [motion_blur]
        enabled      = false
        threshold    = 200.0   # px/s
        max_stretch  = 24.0    # px
        samples      = 4
    """)),

    (".gitignore", textwrap.dedent("""\
        /target
        .ferrum/
        **/*.fe.rs          # transpiled output (future)
        Cargo.lock          # omit for libraries; keep for binaries
    """)),

    ("CHANGELOG.md", textwrap.dedent("""\
        # Changelog

        All notable changes to `fe_ui` are documented here.
        Format follows [Keep a Changelog](https://keepachangelog.com/).

        ## [Unreleased]

        ### Added
        - Initial workspace scaffold
    """)),

    ("CONTRIBUTING.md", textwrap.dedent("""\
        # Contributing to Ferrum

        ## Getting Started

        ```bash
        git clone https://github.com/ferrum-ui/fe_ui
        cd fe_ui
        cargo build
        cargo test --workspace
        ```

        ## Contribution Rules

        1. No cross-subsystem imports — subsystem crates speak `fe_ui_core` only.
        2. No public `EntityId` — never exposed in a public API.
        3. No `GhostWorld` access outside macros.
        4. Frame loop order in `frame.rs` is sacred — RFC required to change.
        5. `fe_ui_core` must compile in < 3s — keep it lean.
        6. `fe_std` uses only the public API.

        ## Where to Start

        - Phase 1: settle/sleep logic — `crates/fe_ui_rapier/src/solver.rs`
        - Phase 1: SDF glyph atlas  — `crates/fe_ui_wgpu/src/text/atlas.rs`
        - Phase 2: gesture tracking — `crates/fe_ui_core/src/input/gesture.rs`

        See [ARCHITECTURE.md](./ARCHITECTURE.md) before touching any crate internals.
    """)),

    ("LICENSE-MIT", "MIT License\n\nCopyright (c) 2024 Ferrum Contributors\n\n"
        "Permission is hereby granted, free of charge, to any person obtaining a copy "
        "of this software...\n"),

    ("LICENSE-APACHE", "Apache License\nVersion 2.0, January 2004\n\n"
        "https://www.apache.org/licenses/LICENSE-2.0\n"),

    # ──────────────────────────────────────────────────────────────────────
    # Assets
    # ──────────────────────────────────────────────────────────────────────
    ("assets/fonts/.gitkeep", ""),
    ("assets/images/.gitkeep", ""),

    ("assets/shaders/sdf_uber.wgsl", textwrap.dedent("""\
        // sdf_uber.wgsl — The Iron-Sight Uber-Shader
        // Handles: rounded rects, circles, squircles, borders, shadows,
        //          glows, gradients, and velocity-based motion blur.
        // Operation order is load-bearing — do not reorder passes.
        //
        // Pass order:
        //   1. Velocity stretch  (motion blur UV warp)
        //   2. SDF evaluation
        //   3. Shadow            (outside shape — must come before fill)
        //   4. Fill              (gradient or solid)
        //   5. Border            (at SDF edge band)
        //   6. Glow              (additive, outside border)
        //   7. Motion blur blend
        //   8. Premultiplied alpha output

        // TODO: implement full shader — see RENDERING_PIPELINE.md for spec
    """)),

    ("assets/shaders/text.wgsl", textwrap.dedent("""\
        // text.wgsl — SDF text rendering pass
        // Consumes the glyph atlas produced by fe_ui_wgpu::text::atlas
        // TODO: implement
    """)),

    ("assets/shaders/portal_stencil.wgsl", textwrap.dedent("""\
        // portal_stencil.wgsl — Stencil mask for Portal clipping
        // Used by ScrollArea, Modal, and any component with overflow clipping.
        // Writes stencil, not colour. Drawn in Pass 1 before the SDF batch.
        // TODO: implement
    """)),

    # ──────────────────────────────────────────────────────────────────────
    # examples/
    # ──────────────────────────────────────────────────────────────────────
    ("examples/bouncing_button.rs", textwrap.dedent("""\
        //! bouncing_button — The "Hello, World" of Ferrum.
        //! Run: cargo run --example bouncing_button
        //!
        //! Demonstrates:
        //!   - App + Window setup
        //!   - A single RigidBody button with restitution
        //!   - on_click impulse
        //!   - use_signal for hover state

        use fe_ui::prelude::*;

        fn main() {
            App::new()
                .add_window(Window::default().title("Bouncing Button"))
                .run(ui);
        }

        #[component]
        fn ui(cx: Scope) -> Element {
            let count     = use_signal(cx, || 0_u32);
            let is_hovered = use_signal(cx, || false);

            render! {
                Flex {
                    direction: Column,
                    gap: 16.0,
                    padding: 32.0,
                    physics_world: PhysicsWorld::default(),

                    Text { size: 32.0, "Count: {count}" }

                    Button {
                        physics: Physics::dynamic()
                            .mass(1.0)
                            .restitution(0.7),
                        on_mouseenter: move |_| is_hovered.set(true),
                        on_mouseleave: move |_| is_hovered.set(false),
                        on_click: move |e| {
                            count.set(*count + 1);
                            e.apply_impulse(Vec2::new(0.0, -400.0));
                        },
                        "Click me ({count})"
                    }
                }
            }
        }
    """)),

    ("examples/modal_drop.rs", textwrap.dedent("""\
        //! modal_drop — Demonstrates the Modal spatial component.
        //! A modal drops from above with gravity and catches on a spring.
        //! Flick to dismiss.
        //!
        //! Run: cargo run --example modal_drop

        use fe_ui::prelude::*;

        fn main() {
            App::new()
                .add_window(Window::default().title("Modal Drop"))
                .run(ui);
        }

        #[component]
        fn ui(cx: Scope) -> Element {
            let show = use_signal(cx, || false);

            render! {
                Flex {
                    direction: Column,
                    align_items: Center,
                    justify_content: Center,
                    physics_world: PhysicsWorld::default(),

                    Button {
                        on_click: move |_| show.set(true),
                        "Open Modal"
                    }

                    Modal {
                        visible: show,
                        on_dismiss: move |_| show.set(false),
                        ModalBody { Text { "Hello from a physical modal!" } }
                    }
                }
            }
        }
    """)),

    ("examples/scroll_physics.rs", textwrap.dedent("""\
        //! scroll_physics — Demonstrates ScrollArea with flick momentum
        //! and rubber-band over-scroll.
        //!
        //! Run: cargo run --example scroll_physics

        use fe_ui::prelude::*;

        fn main() {
            App::new()
                .add_window(Window::default().title("Scroll Physics"))
                .run(ui);
        }

        #[component]
        fn ui(cx: Scope) -> Element {
            render! {
                ScrollArea {
                    direction: ScrollDirection::Vertical,
                    physics: ScrollPhysics {
                        friction: 0.88,
                        overscroll: Overscroll::RubberBand {
                            stiffness: 180.0,
                            max_displacement: 80.0,
                        },
                    },
                    // TODO: populate with list items
                }
            }
        }
    """)),

    ("examples/physics_debug.rs", textwrap.dedent("""\
        //! physics_debug — Runs with show_physics_debug forced on.
        //! Shows Taffy rects, Rapier colliders, sleep states, and
        //! spring force vectors as overlays.
        //!
        //! Run: cargo run --example physics_debug

        use fe_ui::prelude::*;

        fn main() {
            App::new()
                .add_window(Window::default().title("Physics Debug"))
                .configure_renderer(|r| r.show_physics_debug(true))
                .run(ui);
        }

        #[component]
        fn ui(cx: Scope) -> Element {
            render! {
                // TODO: populate with a representative scene
                Flex { direction: Column }
            }
        }
    """)),

    ("examples/custom_shader.rs", textwrap.dedent("""\
        //! custom_shader — Demonstrates a Tier-1 custom .wgsl material
        //! applied to a CustomBox widget. Note: breaks the SDF batch.
        //!
        //! Run: cargo run --example custom_shader

        use fe_ui::prelude::*;

        fn main() {
            App::new()
                .add_window(Window::default().title("Custom Shader"))
                .run(ui);
        }

        #[component]
        fn ui(cx: Scope) -> Element {
            render! {
                CustomBox {
                    shader: "assets/shaders/glow_button.wgsl",
                    style: style! { width: 200.px, height: 60.px },
                }
            }
        }
    """)),

    ("examples/fe_std_showcase.rs", textwrap.dedent("""\
        //! fe_std_showcase — All standard components in one scene.
        //! Use this as a visual regression reference.
        //!
        //! Run: cargo run --example fe_std_showcase

        use fe_ui::prelude::*;
        use fe_std::prelude::*;

        fn main() {
            App::new()
                .add_window(Window::default().title("fe_std Showcase"))
                .run(ui);
        }

        #[component]
        fn ui(cx: Scope) -> Element {
            // TODO: populate with all fe_std components
            render! { Flex { direction: Column } }
        }
    """)),

    # ──────────────────────────────────────────────────────────────────────
    # crates/fe_ui_core
    # ──────────────────────────────────────────────────────────────────────
    ("crates/fe_ui_core/Cargo.toml", textwrap.dedent("""\
        [package]
        name        = "fe_ui_core"
        version     = "0.1.0-alpha"
        edition     = "2021"
        description = "Core types, signals, and the Ghost ECS for Ferrum fe_ui. No GPU. No physics. No layout engine."
        license     = "MIT OR Apache-2.0"

        [dependencies]
        log       = { workspace = true }
        thiserror = { workspace = true }
        slotmap   = { workspace = true }
        glam      = { workspace = true }
    """)),

    ("crates/fe_ui_core/src/lib.rs", textwrap.dedent("""\
        //! fe_ui_core — The shared language of the Ferrum framework.
        //!
        //! No GPU. No physics engine. No layout engine.
        //! Only the types that every subsystem crate speaks.
        //!
        //! # Compile time target
        //! < 3 seconds from clean. Keep this crate lean.
        //! Every dependency added here is paid by every downstream crate.

        pub mod entity;
        pub mod input;
        pub mod physics;
        pub mod signal;
        pub mod style;
        pub mod types;

        pub use entity::{EntityId, GhostWorld};
        pub use physics::PhysicsState;
        pub use signal::{Signal, use_signal, use_layout_signal, use_derived};
        pub use types::{LayoutRect, Transform2D, Color, Vec2, Bounds};
    """)),

    ("crates/fe_ui_core/src/entity.rs", textwrap.dedent("""\
        //! entity.rs — The Ghost ECS.
        //!
        //! GhostEntity maps a single widget across all three subsystems:
        //!   - Taffy NodeId        (layout)
        //!   - Rapier BodyHandle   (physics)
        //!   - wgpu instance index (rendering)
        //!
        //! # Visibility
        //! GhostWorld is pub(crate) only. The #[component] macro is the
        //! only door in. If you need direct entity access, the public API
        //! has a gap — open an issue rather than reaching in here.

        use slotmap::{SlotMap, new_key_type};
        use crate::{style::Style, physics::PhysicsProps, types::LayoutRect};

        new_key_type! {
            /// The only shared identity across layout, physics, and rendering.
            /// Never exposed in a public API.
            pub struct EntityId;
        }

        /// The joint type between a child body and its parent body.
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum JointType {
            /// Default — child is rigidly welded to parent. Moves identically.
            Fixed,
            /// Child follows parent via spring force. Lags, jiggles, settles.
            /// Activated when the child declares Physics::dynamic().
            Spring,
            /// Child is detached. Governed only by its own forces.
            Free,
        }

        impl Default for JointType {
            fn default() -> Self { JointType::Fixed }
        }

        /// Internal representation of a single widget entity.
        /// Holds the cross-system handles and the widget's configuration.
        #[derive(Debug)]
        pub(crate) struct GhostEntity {
            pub id:           EntityId,
            pub taffy_node:   u64,              // taffy::NodeId (opaque)
            pub rapier_body:  u64,              // rapier2d::RigidBodyHandle (opaque)
            pub wgpu_index:   u32,              // index into SDF instance buffer
            pub physics:      PhysicsProps,
            pub style:        Style,
            pub layout_rect:  LayoutRect,       // last resolved Taffy output
            pub children:     Vec<EntityId>,
            pub parent:       Option<EntityId>,
            pub joint:        JointType,
            pub altitude:     f32,
        }

        /// The Ghost ECS — internal entity store.
        /// pub(crate) only. The #[component] macro is the only public door.
        pub(crate) struct GhostWorld {
            entities: SlotMap<EntityId, GhostEntity>,
        }

        impl GhostWorld {
            pub(crate) fn new() -> Self {
                Self { entities: SlotMap::with_key() }
            }

            pub(crate) fn spawn(&mut self, entity: GhostEntity) -> EntityId {
                self.entities.insert(entity)
            }

            pub(crate) fn despawn(&mut self, id: EntityId) -> Option<GhostEntity> {
                self.entities.remove(id)
            }

            pub(crate) fn get(&self, id: EntityId) -> Option<&GhostEntity> {
                self.entities.get(id)
            }

            pub(crate) fn get_mut(&mut self, id: EntityId) -> Option<&mut GhostEntity> {
                self.entities.get_mut(id)
            }

            pub(crate) fn iter(&self) -> impl Iterator<Item = &GhostEntity> {
                self.entities.values()
            }

            pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut GhostEntity> {
                self.entities.values_mut()
            }
        }
    """)),

    ("crates/fe_ui_core/src/types.rs", textwrap.dedent("""\
        //! types.rs — Shared primitive types used across all Ferrum crates.

        use glam::Vec2 as GlamVec2;

        pub use glam::Vec2;

        /// The output of Taffy layout. The input to Rapier spring targets.
        /// The input to wgpu quad sizing.
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct LayoutRect {
            pub x:      f32,
            pub y:      f32,
            pub width:  f32,
            pub height: f32,
        }

        impl LayoutRect {
            pub fn center(&self) -> Vec2 {
                Vec2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
            }

            pub fn size(&self) -> Vec2 {
                Vec2::new(self.width, self.height)
            }

            pub fn contains(&self, point: Vec2) -> bool {
                point.x >= self.x && point.x <= self.x + self.width &&
                point.y >= self.y && point.y <= self.y + self.height
            }
        }

        /// The output of Rapier physics simulation.
        /// Consumed by fe_ui_wgpu after interpolation.
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct Transform2D {
            pub position: Vec2,
            pub rotation: f32,   // radians
            pub scale:    Vec2,
        }

        impl Default for Transform2D {
            fn default() -> Self {
                Self {
                    position: Vec2::ZERO,
                    rotation: 0.0,
                    scale:    Vec2::ONE,
                }
            }
        }

        /// RGBA colour — stored as f32 in [0.0, 1.0].
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct Color {
            pub r: f32,
            pub g: f32,
            pub b: f32,
            pub a: f32,
        }

        impl Color {
            pub const BLACK:       Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
            pub const WHITE:       Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
            pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

            pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
                Self { r, g, b, a }
            }

            /// Parse a hex colour string. Supports "#rrggbb" and "#rrggbbaa".
            pub fn hex(s: &str) -> Self {
                // TODO: implement hex parsing
                Self::BLACK
            }

            pub fn to_array(&self) -> [f32; 4] {
                [self.r, self.g, self.b, self.a]
            }
        }

        /// Axis-aligned bounding box.
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct Bounds {
            pub min: Vec2,
            pub max: Vec2,
        }

        impl Bounds {
            pub fn from_rect(rect: &LayoutRect) -> Self {
                Self {
                    min: Vec2::new(rect.x, rect.y),
                    max: Vec2::new(rect.x + rect.width, rect.y + rect.height),
                }
            }

            pub fn intersects(&self, other: &Bounds) -> bool {
                self.min.x < other.max.x && self.max.x > other.min.x &&
                self.min.y < other.max.y && self.max.y > other.min.y
            }

            pub fn expand(&self, amount: f32) -> Self {
                Self {
                    min: self.min - Vec2::splat(amount),
                    max: self.max + Vec2::splat(amount),
                }
            }
        }
    """)),

    ("crates/fe_ui_core/src/signal.rs", textwrap.dedent("""\
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
    """)),

    ("crates/fe_ui_core/src/physics.rs", textwrap.dedent("""\
        //! physics.rs — Physics property types shared across crates.
        //! fe_ui_rapier reads these; fe_ui_core defines them.
        //! Neither crate imports the other.

        use crate::types::Vec2;

        /// Observable state of a physics body.
        /// Exposed via widget.physics_state() — always available in debug builds.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum PhysicsState {
            /// Body is being stepped. Forces are being applied.
            Active,
            /// Within settle threshold. Decelerating toward target.
            Settling,
            /// At target. Zero CPU cost. Wakes on input or layout change.
            Asleep,
        }

        /// The interaction class of a component.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum InteractionClass {
            /// Spring-driven, pixel-perfect, predictable.
            /// Used by: Switch, Slider, Tabs, Checkbox.
            KinematicSpring,
            /// Impulse-driven, gravity-aware, alive.
            /// Used by: Modal, Toast, Drawer, list items.
            DynamicImpulse,
        }

        /// Physics properties defined by the developer on a component.
        /// Consumed by fe_ui_rapier to configure the Rapier body.
        #[derive(Debug, Clone)]
        pub struct PhysicsProps {
            pub body_type:   BodyType,
            pub mass:        f32,
            pub restitution: f32,       // bounciness — 0.0 = no bounce, 1.0 = perfect
            pub friction:    f32,
            pub drag:        f32,       // linear damping
            pub stiffness:   f32,       // spring stiffness toward layout target
            pub damping:     f32,       // spring damping (1.0 = critically damped)
            pub altitude:    f32,
            pub gravity_scale: f32,
        }

        impl Default for PhysicsProps {
            fn default() -> Self {
                Self {
                    body_type:   BodyType::Kinematic,
                    mass:        1.0,
                    restitution: 0.3,
                    friction:    0.5,
                    drag:        0.0,
                    stiffness:   150.0,
                    damping:     15.0,
                    altitude:    0.0,
                    gravity_scale: 0.0,
                }
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum BodyType {
            /// Controlled by springs toward layout targets. Cannot be pushed by other bodies.
            Kinematic,
            /// Fully simulated. Responds to gravity, impulses, and collisions.
            Dynamic,
            /// No physics body. Rendered at Taffy position only.
            Static,
        }

        /// Stiffness presets — maps to pre-tuned spring coefficients.
        #[derive(Debug, Clone, Copy)]
        pub enum Stiffness {
            Rigid,   // stiffness: 800, damping: 40
            High,    // stiffness: 400, damping: 28
            Medium,  // stiffness: 150, damping: 15  (default)
            Low,     // stiffness:  60, damping:  9
            Fluid,   // stiffness:  20, damping:  5
        }

        impl Stiffness {
            pub fn coefficients(&self) -> (f32, f32) {
                match self {
                    Stiffness::Rigid  => (800.0, 40.0),
                    Stiffness::High   => (400.0, 28.0),
                    Stiffness::Medium => (150.0, 15.0),
                    Stiffness::Low    => (60.0,   9.0),
                    Stiffness::Fluid  => (20.0,   5.0),
                }
            }
        }
    """)),

    ("crates/fe_ui_core/src/style.rs", textwrap.dedent("""\
        //! style.rs — Layout style types (Flexbox / Grid / Block).
        //! Mirrors Taffy's style model but owned by fe_ui_core so no
        //! subsystem crate needs to import taffy directly for types.

        use crate::types::Color;

        #[derive(Debug, Clone, Default)]
        pub struct Style {
            pub display:         Display,
            pub flex_direction:  FlexDirection,
            pub flex_grow:       f32,
            pub flex_shrink:     f32,
            pub align_items:     AlignItems,
            pub justify_content: JustifyContent,
            pub width:           Dimension,
            pub height:          Dimension,
            pub min_width:       Dimension,
            pub min_height:      Dimension,
            pub max_width:       Dimension,
            pub max_height:      Dimension,
            pub padding:         Rect,
            pub margin:          Rect,
            pub gap:             f32,
            pub background:      Color,
            pub border_radius:   f32,
            pub border_width:    f32,
            pub border_color:    Color,
        }

        #[derive(Debug, Clone, Copy, Default)] pub enum Display    { #[default] Flex, Grid, Block, None }
        #[derive(Debug, Clone, Copy, Default)] pub enum FlexDirection { #[default] Row, Column, RowReverse, ColumnReverse }
        #[derive(Debug, Clone, Copy, Default)] pub enum AlignItems  { #[default] Stretch, Center, FlexStart, FlexEnd, Baseline }
        #[derive(Debug, Clone, Copy, Default)] pub enum JustifyContent { #[default] FlexStart, Center, FlexEnd, SpaceBetween, SpaceAround, SpaceEvenly }

        #[derive(Debug, Clone, Copy, Default)]
        pub enum Dimension {
            #[default] Auto,
            Px(f32),
            Percent(f32),
        }

        #[derive(Debug, Clone, Copy, Default)]
        pub struct Rect {
            pub top:    f32,
            pub right:  f32,
            pub bottom: f32,
            pub left:   f32,
        }

        impl Rect {
            pub fn all(v: f32) -> Self { Self { top: v, right: v, bottom: v, left: v } }
        }
    """)),

    ("crates/fe_ui_core/src/input/mod.rs",    stub("crates/fe_ui_core/src/input/mod.rs",    "Input event types and routing")),
    ("crates/fe_ui_core/src/input/gesture.rs", textwrap.dedent("""\
        //! gesture.rs — Gesture velocity tracking and recognition.
        //!
        //! A swipe is not an event; it is an impulse.
        //! The gesture engine tracks touch/mouse velocity and converts
        //! terminal velocity into a Vec2 impulse injected into the physics body.
        //!
        //! Recognised gestures: Swipe, Pinch, LongPress, Drag, Flick.
        //!
        //! TODO: implement velocity ring buffer, gesture state machine,
        //!       impulse calculation from terminal velocity.

        use crate::types::Vec2;

        #[derive(Debug, Clone, Copy)]
        pub enum GestureEvent {
            SwipeUp    { velocity: Vec2 },
            SwipeDown  { velocity: Vec2 },
            SwipeLeft  { velocity: Vec2 },
            SwipeRight { velocity: Vec2 },
            Flick      { velocity: Vec2 },
            Drag       { delta: Vec2, velocity: Vec2 },
            Pinch      { scale_delta: f32 },
            LongPress  { position: Vec2 },
        }
    """)),

    ("crates/fe_ui_core/src/input/keyboard.rs", stub("crates/fe_ui_core/src/input/keyboard.rs", "Keyboard event types and key codes")),
    ("crates/fe_ui_core/src/input/focus.rs",    stub("crates/fe_ui_core/src/input/focus.rs",    "Spatial focus graph and Tab/arrow navigation")),

    # ──────────────────────────────────────────────────────────────────────
    # crates/fe_ui_taffy
    # ──────────────────────────────────────────────────────────────────────
    ("crates/fe_ui_taffy/Cargo.toml", textwrap.dedent("""\
        [package]
        name        = "fe_ui_taffy"
        version     = "0.1.0-alpha"
        edition     = "2021"
        description = "Taffy layout integration for Ferrum fe_ui"
        license     = "MIT OR Apache-2.0"

        [dependencies]
        fe_ui_core = { path = "../fe_ui_core" }
        taffy      = { workspace = true }
        log        = { workspace = true }
    """)),

    ("crates/fe_ui_taffy/src/lib.rs", textwrap.dedent("""\
        //! fe_ui_taffy — Layout subsystem for Ferrum.
        //!
        //! Wraps Taffy. Owns the layout tree. Produces LayoutRect outputs.
        //!
        //! # Contract
        //! - Receives a dirty EntityId set at the Sync Point.
        //! - Recomputes only affected subtrees (never the full tree).
        //! - Outputs HashMap<EntityId, LayoutRect> — resolved target positions.
        //! - Does NOT know about Rapier or wgpu.

        pub mod cache;
        pub mod convert;
        pub mod tree;

        pub use tree::LayoutTree;
    """)),

    ("crates/fe_ui_taffy/src/tree.rs", textwrap.dedent("""\
        //! tree.rs — The Ferrum layout tree, backed by Taffy.
        //!
        //! Maintains a Taffy node tree mirroring the GhostWorld entity tree.
        //! Called once per frame at the Sync Point with the dirty entity set.

        use std::collections::HashMap;
        use fe_ui_core::{EntityId, types::LayoutRect, style::Style};

        pub struct LayoutTree {
            // TODO: taffy::TaffyTree
        }

        impl LayoutTree {
            pub fn new() -> Self { Self {} }

            /// Register a new entity and its style with the layout tree.
            pub fn register(&mut self, id: EntityId, style: &Style) {
                // TODO: create taffy node, store EntityId → NodeId mapping
            }

            /// Remove an entity from the layout tree.
            pub fn unregister(&mut self, id: EntityId) {
                // TODO: remove taffy node
            }

            /// Recompute layout for dirty entities and return resolved rects.
            /// Called once per frame at the Sync Point.
            pub fn compute(
                &mut self,
                dirty:  &[EntityId],
                styles: &HashMap<EntityId, Style>,
            ) -> HashMap<EntityId, LayoutRect> {
                // TODO:
                //   1. Apply updated styles for dirty nodes.
                //   2. Call taffy.compute_layout() for affected subtrees.
                //   3. Walk results, convert to LayoutRect, return map.
                HashMap::new()
            }
        }
    """)),

    ("crates/fe_ui_taffy/src/cache.rs",   stub("crates/fe_ui_taffy/src/cache.rs",   "Layout result cache — invalidated by dirty signals")),
    ("crates/fe_ui_taffy/src/convert.rs", stub("crates/fe_ui_taffy/src/convert.rs", "fe_ui Style → taffy Style conversion")),

    # ──────────────────────────────────────────────────────────────────────
    # crates/fe_ui_rapier
    # ──────────────────────────────────────────────────────────────────────
    ("crates/fe_ui_rapier/Cargo.toml", textwrap.dedent("""\
        [package]
        name        = "fe_ui_rapier"
        version     = "0.1.0-alpha"
        edition     = "2021"
        description = "Rapier2D physics integration for Ferrum fe_ui"
        license     = "MIT OR Apache-2.0"

        [dependencies]
        fe_ui_core = { path = "../fe_ui_core" }
        rapier2d   = { workspace = true }
        log        = { workspace = true }
        glam       = { workspace = true }
    """)),

    ("crates/fe_ui_rapier/src/lib.rs", textwrap.dedent("""\
        //! fe_ui_rapier — Physics subsystem for Ferrum.
        //!
        //! Wraps Rapier2D. Owns the physics world.
        //! Produces Transform2D + PhysicsState + velocity outputs.
        //!
        //! # Contract
        //! - Receives new LayoutRect spring targets at the Sync Point.
        //! - Steps the physics world at 240Hz with configurable substeps.
        //! - Outputs per-entity (Transform2D, PhysicsState, Vec2 velocity).
        //! - Does NOT know about Taffy or wgpu.
        //! - Physics events are queued as signals for the next frame — never
        //!   written back into the current physics step.

        pub mod collider;
        pub mod gesture;
        pub mod joints;
        pub mod portal;
        pub mod solver;
        pub mod world;

        pub use world::PhysicsWorld;
    """)),

    ("crates/fe_ui_rapier/src/world.rs", textwrap.dedent("""\
        //! world.rs — The Ferrum physics world, backed by Rapier2D.

        use std::collections::HashMap;
        use fe_ui_core::{EntityId, physics::PhysicsProps, types::{LayoutRect, Transform2D, Vec2}};

        pub struct PhysicsWorld {
            // TODO: rapier2d RigidBodySet, ColliderSet, ImpulseJointSet,
            //       IntegrationParameters, PhysicsPipeline, etc.
            config: WorldConfig,
        }

        pub struct WorldConfig {
            pub gravity:           Vec2,
            pub air_density:       f32,
            pub pixels_per_meter:  f32,
            pub physics_hz:        f32,
            pub substeps:          u32,
            pub solver_iterations: u32,
        }

        impl Default for WorldConfig {
            fn default() -> Self {
                Self {
                    gravity:          Vec2::new(0.0, 980.0),
                    air_density:      0.1,
                    pixels_per_meter: 100.0,
                    physics_hz:       240.0,
                    substeps:         4,
                    solver_iterations: 8,
                }
            }
        }

        impl PhysicsWorld {
            pub fn new(config: WorldConfig) -> Self {
                Self { config }
            }

            /// Spawn a physics body for a new entity.
            pub fn spawn_body(&mut self, id: EntityId, props: &PhysicsProps) {
                // TODO: create Rapier rigid body + collider
            }

            /// Remove a physics body.
            pub fn despawn_body(&mut self, id: EntityId) {
                // TODO: remove from Rapier sets
            }

            /// Update spring targets from the Sync Point's Taffy output.
            pub fn update_spring_targets(&mut self, targets: &HashMap<EntityId, LayoutRect>) {
                // TODO: call solver::update_spring_targets
            }

            /// Apply a queued impulse to a body.
            pub fn apply_impulse(&mut self, id: EntityId, impulse: Vec2) {
                // TODO
            }

            /// Step the physics world. Returns per-entity transforms.
            /// Collision events are queued into signal_queue for the next frame.
            pub fn step(&mut self) -> HashMap<EntityId, (Transform2D, Vec2)> {
                // TODO: run Rapier pipeline × substeps, collect results
                HashMap::new()
            }
        }
    """)),

    ("crates/fe_ui_rapier/src/solver.rs", textwrap.dedent("""\
        //! solver.rs — The Spring Constraint Solver.
        //!
        //! THE most critical file in the Ferrum codebase.
        //! This is the bridge between Taffy's "where things should be"
        //! and Rapier's "where things are."
        //!
        //! # The Contract
        //! Bodies never teleport. They always arrive at their Taffy target
        //! via spring force. The spring target is the only communication
        //! channel between the layout system and the physics system.
        //!
        //! # Settle Logic
        //! When a body is within SETTLE_THRESHOLD_PX of its target AND
        //! velocity magnitude is below SLEEP_VELOCITY_THRESHOLD, the body
        //! is hard-snapped and removed from the active step queue.
        //! This is the primary battery/CPU optimisation.

        use fe_ui_core::types::Vec2;

        /// Distance threshold for entering the settling state.
        pub const SETTLE_THRESHOLD_PX:    f32 = 0.5;
        /// Distance threshold for hard-snap and sleep.
        pub const SLEEP_THRESHOLD_PX:     f32 = 0.1;
        /// Velocity threshold for sleep.
        pub const SLEEP_VELOCITY_THRESHOLD: f32 = 1.0; // px/s

        // TODO: implement
        //   - update_spring_targets(bodies, joints, targets, handles)
        //   - evaluate_sleep(bodies, targets, handles) -> Vec<EntityId> (newly asleep)
        //   - wake_body(id, bodies, handles)
    """)),

    ("crates/fe_ui_rapier/src/joints.rs",  stub("crates/fe_ui_rapier/src/joints.rs",  "Fixed, Spring, Free joint implementations")),
    ("crates/fe_ui_rapier/src/collider.rs",stub("crates/fe_ui_rapier/src/collider.rs","Collider shapes: rect, circle, squircle")),
    ("crates/fe_ui_rapier/src/gesture.rs", stub("crates/fe_ui_rapier/src/gesture.rs", "Gesture velocity → impulse injection into Rapier bodies")),
    ("crates/fe_ui_rapier/src/portal.rs",  stub("crates/fe_ui_rapier/src/portal.rs",  "Portal boundary colliders — rigid and spring types")),

    # ──────────────────────────────────────────────────────────────────────
    # crates/fe_ui_wgpu
    # ──────────────────────────────────────────────────────────────────────
    ("crates/fe_ui_wgpu/Cargo.toml", textwrap.dedent("""\
        [package]
        name        = "fe_ui_wgpu"
        version     = "0.1.0-alpha"
        edition     = "2021"
        description = "wgpu rendering pipeline for Ferrum fe_ui — Iron-Sight SDF renderer"
        license     = "MIT OR Apache-2.0"

        [dependencies]
        fe_ui_core  = { path = "../fe_ui_core" }
        wgpu        = { workspace = true }
        winit       = { workspace = true }
        bytemuck    = { workspace = true }
        cosmic-text = { workspace = true }
        glam        = { workspace = true }
        log         = { workspace = true }
    """)),

    ("crates/fe_ui_wgpu/src/lib.rs", textwrap.dedent("""\
        //! fe_ui_wgpu — Rendering subsystem for Ferrum. The Iron-Sight Pipeline.
        //!
        //! Owns the wgpu device, surface, and render pipeline.
        //! Consumes Transform2D (from Rapier, interpolated) and LayoutRect
        //! (from Taffy, for quad sizing). Produces pixels.
        //!
        //! # Draw call budget
        //! - Pass 1: portal stencil masks  (1 per portal boundary)
        //! - Pass 2: Tier 0 SDF batch      (1 draw call — all standard primitives)
        //! - Pass 3: Tier 1 custom shaders (1 per unique .wgsl material)
        //! - Pass 4: composite + tonemap
        //!
        //! # Contract
        //! Does not run physics. Does not run layout. Exposes render(frame).

        pub mod atlas;
        pub mod instance;
        pub mod interpolate;
        pub mod pipeline;
        pub mod portal;
        pub mod renderer;
        pub mod text;

        pub use renderer::Renderer;
    """)),

    ("crates/fe_ui_wgpu/src/interpolate.rs", textwrap.dedent("""\
        //! interpolate.rs — Physics interpolation for smooth rendering.
        //!
        //! The physics world steps at 240Hz. The renderer draws at 60/144Hz.
        //! Without interpolation, fast-moving widgets stutter.
        //!
        //! # The Alpha
        //!   alpha = (frame_time - last_physics_step_time) / physics_step_duration
        //!   render_pos = lerp(prev_pos, curr_pos, alpha)
        //!   render_rot = slerp(prev_rot, curr_rot, alpha)  // slerp for correctness
        //!
        //! Getting alpha wrong produces a half-frame visual lag that feels like
        //! input latency even when measured latency is fine.

        use fe_ui_core::types::{Transform2D, Vec2};

        pub fn interpolate_transform(
            prev:  &Transform2D,
            curr:  &Transform2D,
            alpha: f32,
        ) -> Transform2D {
            Transform2D {
                position: prev.position.lerp(curr.position, alpha),
                rotation: lerp_angle(prev.rotation, curr.rotation, alpha),
                scale:    prev.scale.lerp(curr.scale, alpha),
            }
        }

        fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
            // TODO: implement shortest-path angle interpolation
            a + (b - a) * t
        }

        /// Extract render-space velocity for motion blur.
        /// pixels_per_meter converts from Rapier metres to screen pixels.
        pub fn render_velocity(
            prev_pos:        Vec2,
            curr_pos:        Vec2,
            physics_hz:      f32,
            pixels_per_meter: f32,
        ) -> Vec2 {
            (curr_pos - prev_pos) * physics_hz * pixels_per_meter
        }
    """)),

    ("crates/fe_ui_wgpu/src/instance.rs", textwrap.dedent("""\
        //! instance.rs — SDF instance buffer packing.
        //!
        //! One SdfInstance per Tier-0 widget. Packed into a GPU buffer.
        //! One instanced draw call covers the entire standard UI.
        //!
        //! Operation order inside the uber-shader is load-bearing:
        //!   1. Velocity stretch  2. SDF eval  3. Shadow  4. Fill
        //!   5. Border  6. Glow  7. Motion blur blend  8. Premultiplied alpha

        use bytemuck::{Pod, Zeroable};

        /// Per-instance data uploaded to the GPU each frame.
        /// Must match the struct layout in sdf_uber.wgsl exactly.
        #[repr(C)]
        #[derive(Debug, Clone, Copy, Pod, Zeroable)]
        pub struct SdfInstance {
            pub transform:     [[f32; 3]; 3], // mat3x3 (position, rotation, scale)
            pub size:          [f32; 2],
            pub corner_radius: f32,
            pub shape_type:    u32,           // 0=rect 1=circle 2=squircle 3=text_quad
            pub velocity:      [f32; 2],      // for motion blur
            pub altitude:      f32,           // for shadow derivation
            pub z_depth:       f32,           // depth buffer value
            pub color:         [f32; 4],
            pub border_color:  [f32; 4],
            pub border_width:  f32,
            pub glow_color:    [f32; 4],
            pub glow_radius:   f32,
            pub _pad:          [f32; 2],      // alignment
        }

        // TODO: implement instance buffer, upload, and per-frame packing
    """)),

    ("crates/fe_ui_wgpu/src/renderer.rs", stub("crates/fe_ui_wgpu/src/renderer.rs", "Top-level Renderer — owns wgpu device/surface, runs draw passes")),
    ("crates/fe_ui_wgpu/src/pipeline.rs", stub("crates/fe_ui_wgpu/src/pipeline.rs", "wgpu render pipeline construction and shader compilation")),
    ("crates/fe_ui_wgpu/src/portal.rs",   stub("crates/fe_ui_wgpu/src/portal.rs",   "Stencil mask rendering for Portal clipping boundaries")),
    ("crates/fe_ui_wgpu/src/atlas.rs",    textwrap.dedent("""\
        //! atlas.rs — SDF glyph atlas.
        //!
        //! Manages the GPU texture atlas for SDF-rendered glyphs.
        //! Uses cosmic-text for shaping and an LRU cache for eviction.
        //!
        //! Goals:
        //!   - Crisp text at any scale (SDF, no re-rasterisation on resize)
        //!   - Full RTL, complex shaping, font-fallback via cosmic-text
        //!   - < 1% atlas miss rate for common glyph sets
        //!   - Memory-bounded LRU eviction

        // TODO: implement GlyphAtlas with LRU eviction
        //   - rasterise glyphs via cosmic-text → SDF
        //   - pack into wgpu Texture
        //   - provide UV rect lookups for the text shader
    """)),

    ("crates/fe_ui_wgpu/src/text/mod.rs",     stub("crates/fe_ui_wgpu/src/text/mod.rs",     "Text rendering module")),
    ("crates/fe_ui_wgpu/src/text/shaper.rs",  stub("crates/fe_ui_wgpu/src/text/shaper.rs",  "cosmic-text shaping integration")),
    ("crates/fe_ui_wgpu/src/text/layout.rs",  stub("crates/fe_ui_wgpu/src/text/layout.rs",  "Text layout — wrapping, RTL, line breaking")),

    # ──────────────────────────────────────────────────────────────────────
    # crates/fe_ui  (orchestrator)
    # ──────────────────────────────────────────────────────────────────────
    ("crates/fe_ui/Cargo.toml", textwrap.dedent("""\
        [package]
        name        = "fe_ui"
        version     = "0.1.0-alpha"
        edition     = "2021"
        description = "Ferrum fe_ui — GPU-accelerated Rust UI framework with physics-native layout"
        license     = "MIT OR Apache-2.0"

        [dependencies]
        fe_ui_core   = { path = "../fe_ui_core" }
        fe_ui_taffy  = { path = "../fe_ui_taffy" }
        fe_ui_rapier = { path = "../fe_ui_rapier" }
        fe_ui_wgpu   = { path = "../fe_ui_wgpu" }

        winit      = { workspace = true }
        log        = { workspace = true }
        accesskit  = { workspace = true }
    """)),

    ("crates/fe_ui/src/lib.rs", textwrap.dedent("""\
        //! fe_ui — The Ferrum framework orchestrator.
        //!
        //! This is the only crate that imports all three subsystems.
        //! It owns the frame loop, the Sync Point, and the event system.
        //!
        //! # The frame loop (frame.rs) execution order is sacred.
        //! Reordering steps has non-obvious physics and rendering consequences.
        //! An RFC is required to change it.

        pub mod app;
        pub mod event;
        pub mod frame;
        pub mod macros;
        pub mod sync;
        pub mod window;

        pub mod prelude {
            pub use crate::app::App;
            pub use crate::window::Window;
            pub use fe_ui_core::{
                entity::EntityId,
                physics::{PhysicsProps, PhysicsState, Stiffness, BodyType, InteractionClass},
                signal::{Signal, LayoutSignal, use_signal, use_layout_signal, use_derived},
                style::{Style, Dimension, FlexDirection, AlignItems, JustifyContent},
                types::{Color, LayoutRect, Transform2D, Vec2, Bounds},
            };
        }
    """)),

    ("crates/fe_ui/src/frame.rs", textwrap.dedent("""\
        //! frame.rs — The Ferrum frame loop.
        //!
        //! THE execution order defined here is sacred.
        //! Do not reorder steps without an RFC.
        //! Reordering has non-obvious physics and rendering consequences.
        //!
        //! Frame execution order:
        //!
        //!   1.  Drain input events → signals fire, impulses queued
        //!   ═══ SYNC POINT ══════════════════════════════════════
        //!   2.  Drain queued physics signals from previous frame
        //!   3.  Resolve dirty derived signals (topological order)
        //!   4.  Collect dirty layout entity set
        //!   5.  Taffy recomputes affected subtrees — once, atomically
        //!   6.  Diff targets — only changed nodes update springs
        //!   7.  Inject new spring targets into Rapier constraints
        //!   ═════════════════════════════════════════════════════
        //!   8.  Drain input impulses → inject into Rapier bodies
        //!   9.  Step physics (240Hz internal, substeps × 4)
        //!         Collision events → queued for next frame
        //!  10.  Render
        //!         Interpolate transforms
        //!         Pack SDF instance buffer
        //!         Draw (portal stencil → Tier 0 batch → Tier 1 custom)

        // TODO: implement run_frame(app, layout, physics, renderer, signals)
    """)),

    ("crates/fe_ui/src/sync.rs", textwrap.dedent("""\
        //! sync.rs — The Sync Point.
        //!
        //! The single boundary where the signal world hands off to the physics world.
        //! Runs once per frame, between input processing and the physics step.
        //!
        //! Signals are Intent. Physics is Reality. The Sync Point is where
        //! Intent becomes Reality.
        //!
        //! See SIGNAL_GRAPH.md for the full specification.

        // TODO: implement SyncPoint::run(signals, layout, physics)
        //   Steps 2–7 of the frame loop live here.
    """)),

    ("crates/fe_ui/src/app.rs",    stub("crates/fe_ui/src/app.rs",    "App builder — window config, subsystem init, run()")),
    ("crates/fe_ui/src/window.rs", stub("crates/fe_ui/src/window.rs", "Window configuration and winit integration")),
    ("crates/fe_ui/src/event.rs",  stub("crates/fe_ui/src/event.rs",  "Input event routing — signals, impulses, gesture dispatch")),
    ("crates/fe_ui/src/macros.rs", textwrap.dedent("""\
        //! macros.rs — The #[component] macro and render! DSL.
        //!
        //! The #[component] macro is the only public door into the Ghost ECS.
        //! It handles all three subsystem registrations invisibly:
        //!
        //!   - Taffy registration (layout)
        //!   - Rapier body spawning (physics)
        //!   - wgpu instance reservation (rendering)
        //!
        //! The developer writes Style + Physics. The macro wires everything else.
        //! A component should never have to manually update its position.
        //!
        //! See ARCHITECTURE.md § The #[component] Macro for the expansion spec.

        // TODO: implement proc-macro crate (fe_ui_macros) and re-export here.
        //   The macro expansion registers the entity across all three subsystems.
    """)),

    # ──────────────────────────────────────────────────────────────────────
    # crates/fe_std
    # ──────────────────────────────────────────────────────────────────────
    ("crates/fe_std/Cargo.toml", textwrap.dedent("""\
        [package]
        name        = "fe_std"
        version     = "0.1.0-alpha"
        edition     = "2021"
        description = "Standard physics-tuned component library for Ferrum fe_ui"
        license     = "MIT OR Apache-2.0"

        [dependencies]
        fe_ui = { path = "../fe_ui" }

        # fe_std uses ONLY the fe_ui public API.
        # If building a component requires internal access, fix fe_ui first.
    """)),

    ("crates/fe_std/src/lib.rs", textwrap.dedent("""\
        //! fe_std — Standard component library for Ferrum.
        //!
        //! # Philosophy: "Predictable Controls, Alive Containers."
        //!
        //! Kinematic-Spring (Precision Controls):
        //!   Switch, Slider, Tabs, Checkbox
        //!   → Spring-driven, pixel-perfect, predictable, trustworthy.
        //!   → Critically damped by default — zero overshoot.
        //!
        //! Dynamic-Impulse (Spatial Components):
        //!   Modal, Toast, Drawer, ScrollArea, list items
        //!   → Gravity + impulse driven, alive, flick-able.
        //!   → Reward the user's input velocity.
        //!
        //! # Theming Contract
        //! All standard components expose a Theme struct that maps to SDF uniforms.
        //! Everything stays Tier 0 — one draw call for the entire fe_std library.
        //! Custom shaders use CustomBox { shader: "..." } explicitly.
        //!
        //! # Batch Tier
        //! Every component in this crate is Tier 0. If you're adding a component
        //! that requires a custom shader, it does not belong in fe_std.

        pub mod button;
        pub mod checkbox;
        pub mod drawer;
        pub mod modal;
        pub mod scroll_area;
        pub mod slider;
        pub mod switch;
        pub mod tab_bar;
        pub mod text;
        pub mod toast;

        pub mod prelude {
            pub use crate::{
                button::Button,
                checkbox::Checkbox,
                drawer::Drawer,
                modal::Modal,
                scroll_area::ScrollArea,
                slider::Slider,
                switch::Switch,
                tab_bar::TabBar,
                text::Text,
                toast::Toast,
            };
        }
    """)),

    ("crates/fe_std/src/button.rs", textwrap.dedent("""\
        //! button.rs — The Button component.
        //!
        //! Class: Kinematic-Spring (press deformation) + Dynamic-Impulse on click.
        //!
        //! Physics lifecycle:
        //!   At rest   → kinematic body at Taffy target, sleep state.
        //!   Hover     → wake, glow_radius spring → 8.0, altitude spring → 3.0.
        //!   Press     → scale spring → 0.92, altitude spring → 1.0.
        //!   Release   → impulse applied, scale spring → 1.0, altitude → 2.0, settle.
        //!
        //! Tier 0 — no custom shader. Part of the SDF batch.
        //!
        //! See FE_STD_SPEC.md § Button for full specification.

        use fe_ui::prelude::*;

        #[derive(Debug, Clone)]
        pub struct ButtonTheme {
            pub color:         Color,
            pub color_hovered: Color,
            pub color_pressed: Color,
            pub border_color:  Color,
            pub border_width:  f32,
            pub corner_radius: f32,
            pub glow_color:    Color,
            pub glow_radius:   f32,
            pub altitude:      f32,
        }

        impl Default for ButtonTheme {
            fn default() -> Self {
                Self {
                    color:         Color::hex("#7c3aed"),
                    color_hovered: Color::hex("#6d28d9"),
                    color_pressed: Color::hex("#5b21b6"),
                    border_color:  Color::hex("#a78bfa"),
                    border_width:  1.5,
                    corner_radius: 10.0,
                    glow_color:    Color::hex("#7c3aed"),
                    glow_radius:   0.0,
                    altitude:      2.0,
                }
            }
        }

        // TODO: implement #[component] Button
    """)),

    ("crates/fe_std/src/switch.rs", textwrap.dedent("""\
        //! switch.rs — The Switch component (toggle).
        //!
        //! Class: Kinematic-Spring (Precision Control).
        //!
        //! Critically damped by default (damping: 1.0) — zero overshoot.
        //! The thumb always arrives at ON or OFF. Never ambiguous.
        //! This is the difference between a control and a toy.
        //!
        //! See FE_STD_SPEC.md § Switch for full specification.

        // TODO: implement #[component] Switch
    """)),

    ("crates/fe_std/src/slider.rs", textwrap.dedent("""\
        //! slider.rs — The Slider component.
        //!
        //! Class: Kinematic-Spring (Precision Control).
        //!
        //! The thumb has mass (default 0.8). Dragging feels weighted.
        //! On step release, a small directional impulse is applied before
        //! the spring seeks the snap target — the thumb *snaps*, not floats.
        //!
        //! See FE_STD_SPEC.md § Slider for full specification.

        // TODO: implement #[component] Slider
    """)),

    ("crates/fe_std/src/modal.rs", textwrap.dedent("""\
        //! modal.rs — The Modal component.
        //!
        //! Class: Dynamic-Impulse (Spatial Component).
        //!
        //! Entry: spawned above viewport, falls with gravity, catches on spring at center.
        //! Exit:  flick to dismiss (velocity threshold), or programmatic.
        //! Children: fixed-jointed by default; Physics::dynamic() → spring joint (jiggle).
        //!
        //! See FE_STD_SPEC.md § Modal for full specification.

        // TODO: implement #[component] Modal
    """)),

    ("crates/fe_std/src/scroll_area.rs", textwrap.dedent("""\
        //! scroll_area.rs — The ScrollArea component.
        //!
        //! Class: Dynamic-Impulse (Spatial Container).
        //!
        //! Flick applies a real velocity impulse. Edges are spring colliders
        //! (rubber-band over-scroll). Content body sleeps when velocity drops
        //! below threshold — zero CPU cost at rest.
        //!
        //! See FE_STD_SPEC.md § ScrollArea for full specification.

        // TODO: implement #[component] ScrollArea
    """)),

    ("crates/fe_std/src/toast.rs",    stub("crates/fe_std/src/toast.rs",    "Toast — Dynamic-Impulse notification. Slides in, falls out.")),
    ("crates/fe_std/src/drawer.rs",   stub("crates/fe_std/src/drawer.rs",   "Drawer — Dynamic-Impulse edge panel. Velocity-aware open/close.")),
    ("crates/fe_std/src/tab_bar.rs",  stub("crates/fe_std/src/tab_bar.rs",  "TabBar — Kinematic-Spring. Indicator stretches during travel.")),
    ("crates/fe_std/src/checkbox.rs", stub("crates/fe_std/src/checkbox.rs", "Checkbox — Kinematic-Spring. Check mark spring-draws on toggle.")),
    ("crates/fe_std/src/text.rs",     stub("crates/fe_std/src/text.rs",     "Text — SDF-rendered, cosmic-text shaped, glyph-atlas backed.")),

    # ──────────────────────────────────────────────────────────────────────
    # .ferrum/ (tooling cache — gitignored)
    # ──────────────────────────────────────────────────────────────────────
    (".ferrum/cache/glyphs/.gitkeep",   ""),
    (".ferrum/cache/shaders/.gitkeep",  ""),
    (".ferrum/cache/assets.ron",        "// Asset manifest — generated by build.rs. Do not edit manually.\n"),

    # ──────────────────────────────────────────────────────────────────────
    # build.rs
    # ──────────────────────────────────────────────────────────────────────
    ("build.rs", textwrap.dedent("""\
        //! build.rs — Ferrum asset pipeline.
        //!
        //! Runs at compile time to:
        //!   1. Validate .wgsl shaders via naga (type-check at compile time,
        //!      not at runtime — invalid shader = compile error).
        //!   2. Bundle fonts, images into optimised formats.
        //!   3. Generate src/assets.rs — strongly typed asset handles.
        //!      Missing asset file = compile error, not a runtime panic.
        //!
        //! Keeps compile times low by doing expensive validation once,
        //! cached in .ferrum/cache/. Only re-runs when source assets change.

        fn main() {
            // Re-run if any asset changes
            println!("cargo:rerun-if-changed=assets/");
            println!("cargo:rerun-if-changed=ferrum.toml");

            // TODO: implement
            //   ferrum_build::AssetPipeline::new()
            //       .validate_shaders("assets/shaders/")
            //       .bundle_fonts("assets/fonts/")
            //       .stage_images("assets/images/")
            //       .emit_typed_handles()   // generates src/assets.rs
            //       .run();
        }
    """)),
]


# ──────────────────────────────────────────────────────────────────────────────
# SCAFFOLDER
# ──────────────────────────────────────────────────────────────────────────────

def scaffold(root: Path) -> None:
    if not (root / "README.md").exists():
        print(f"\n{YELLOW}WARNING: README.md not found in {root}{RESET}")
        answer = input("  Continue anyway? [y/N] ").strip().lower()
        if answer != "y":
            sys.exit(0)

    header(f"Ferrum fe_ui — Scaffolding into: {root}")

    created_dirs:  list[Path] = []
    created_files: list[Path] = []
    skipped_files: list[Path] = []

    for rel_path, content in FILES:
        target = root / rel_path
        target.parent.mkdir(parents=True, exist_ok=True)

        if target.parent not in created_dirs and target.parent != root:
            created_dirs.append(target.parent)

        if target.exists():
            skipped_files.append(target)
            warn(f"EXISTS  {rel_path}")
            continue

        target.write_text(content, encoding="utf-8")
        created_files.append(target)
        ok(f"CREATE  {rel_path}")

    # Summary
    header("─── Summary ───────────────────────────────────────────")
    print(f"  {GREEN}Created : {len(created_files)} files{RESET}")
    if skipped_files:
        print(f"  {YELLOW}Skipped : {len(skipped_files)} files (already exist){RESET}")
    print()
    print(f"  {CYAN}Next steps:{RESET}")
    print(f"    cd {root}")
    print(f"    cargo build")
    print(f"    cargo run --example bouncing_button")
    print()
    print(f"  {BOLD}Built different.{RESET}\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Scaffold the Ferrum fe_ui framework directory tree.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path.cwd(),
        metavar="PATH",
        help="Directory to scaffold into (default: current directory). "
             "Should already contain README.md.",
    )
    return parser.parse_args()


if __name__ == "__main__":
    args = parse_args()
    scaffold(args.root.resolve())
