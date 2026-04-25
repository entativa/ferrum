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

        use fe_ui::prelude::*;

        fn main() {
            App::new()
                .add_window(Window::default().title("Bouncing Button"))
                .run(ui);
        }

        #[component]
        fn ui(cx: Scope) -> Element {
            let count      = use_signal(cx, || 0_u32);
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
        //! scroll_physics — Demonstrates ScrollArea with flick momentum.
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
                Flex { direction: Column }
            }
        }
    """)),

    ("examples/custom_shader.rs", textwrap.dedent("""\
        //! custom_shader — Demonstrates a Tier-1 custom .wgsl material.
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
        description = "Core types, signals, and the Ghost ECS for Ferrum fe_ui."
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

        use slotmap::{SlotMap, new_key_type};
        use crate::{style::Style, physics::PhysicsProps, types::LayoutRect};

        new_key_type! {
            pub struct EntityId;
        }

        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum JointType {
            Fixed,
            Spring,
            Free,
        }

        impl Default for JointType {
            fn default() -> Self { JointType::Fixed }
        }

        #[derive(Debug)]
        pub(crate) struct GhostEntity {
            pub id:          EntityId,
            pub taffy_node:  u64,
            pub rapier_body: u64,
            pub wgpu_index:  u32,
            pub physics:     PhysicsProps,
            pub style:       Style,
            pub layout_rect: LayoutRect,
            pub children:    Vec<EntityId>,
            pub parent:      Option<EntityId>,
            pub joint:       JointType,
            pub altitude:    f32,
        }

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

        pub use glam::Vec2;

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

        /// NOTE: Default is implemented manually so scale initialises to
        /// Vec2::ONE rather than Vec2::ZERO. Do not add #[derive(Default)].
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct Transform2D {
            pub position: Vec2,
            pub rotation: f32,
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

            pub fn hex(_s: &str) -> Self {
                // TODO: implement hex parsing
                Self::BLACK
            }

            pub fn to_array(&self) -> [f32; 4] {
                [self.r, self.g, self.b, self.a]
            }
        }

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

        pub struct Signal<T>(pub T);
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

        use crate::types::Vec2;

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum PhysicsState {
            Active,
            Settling,
            Asleep,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum InteractionClass {
            KinematicSpring,
            DynamicImpulse,
        }

        #[derive(Debug, Clone)]
        pub struct PhysicsProps {
            pub body_type:     BodyType,
            pub mass:          f32,
            pub restitution:   f32,
            pub friction:      f32,
            pub drag:          f32,
            pub stiffness:     f32,
            pub damping:       f32,
            pub altitude:      f32,
            pub gravity_scale: f32,
        }

        impl Default for PhysicsProps {
            fn default() -> Self {
                Self {
                    body_type:     BodyType::Kinematic,
                    mass:          1.0,
                    restitution:   0.3,
                    friction:      0.5,
                    drag:          0.0,
                    stiffness:     150.0,
                    damping:       15.0,
                    altitude:      0.0,
                    gravity_scale: 0.0,
                }
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum BodyType {
            Kinematic,
            Dynamic,
            Static,
        }

        #[derive(Debug, Clone, Copy)]
        pub enum Stiffness {
            Rigid,
            High,
            Medium,
            Low,
            Fluid,
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

        #[derive(Debug, Clone, Copy, Default)] pub enum Display       { #[default] Flex, Grid, Block, None }
        #[derive(Debug, Clone, Copy, Default)] pub enum FlexDirection  { #[default] Row, Column, RowReverse, ColumnReverse }
        #[derive(Debug, Clone, Copy, Default)] pub enum AlignItems     { #[default] Stretch, Center, FlexStart, FlexEnd, Baseline }
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

    # input/ — stub() already returns (path, content), so no outer tuple needed
    stub("crates/fe_ui_core/src/input/mod.rs",     "Input event types and routing"),
    stub("crates/fe_ui_core/src/input/keyboard.rs","Keyboard event types and key codes"),
    stub("crates/fe_ui_core/src/input/focus.rs",   "Spatial focus graph and Tab/arrow navigation"),

    ("crates/fe_ui_core/src/input/gesture.rs", textwrap.dedent("""\
        //! gesture.rs — Gesture velocity tracking and recognition.

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

        pub mod cache;
        pub mod convert;
        pub mod tree;

        pub use tree::LayoutTree;
    """)),

    ("crates/fe_ui_taffy/src/tree.rs", textwrap.dedent("""\
        //! tree.rs — The Ferrum layout tree, backed by Taffy.

        use std::collections::HashMap;
        use fe_ui_core::{EntityId, types::LayoutRect, style::Style};

        pub struct LayoutTree {}

        impl LayoutTree {
            pub fn new() -> Self { Self {} }

            pub fn register(&mut self, _id: EntityId, _style: &Style) {
                // TODO: create taffy node, store EntityId → NodeId mapping
            }

            pub fn unregister(&mut self, _id: EntityId) {
                // TODO: remove taffy node
            }

            pub fn compute(
                &mut self,
                _dirty:  &[EntityId],
                _styles: &HashMap<EntityId, Style>,
            ) -> HashMap<EntityId, LayoutRect> {
                HashMap::new()
            }
        }
    """)),

    stub("crates/fe_ui_taffy/src/cache.rs",   "Layout result cache — invalidated by dirty signals"),
    stub("crates/fe_ui_taffy/src/convert.rs", "fe_ui Style → taffy Style conversion"),

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
                    gravity:           Vec2::new(0.0, 980.0),
                    air_density:       0.1,
                    pixels_per_meter:  100.0,
                    physics_hz:        240.0,
                    substeps:          4,
                    solver_iterations: 8,
                }
            }
        }

        impl PhysicsWorld {
            pub fn new(config: WorldConfig) -> Self { Self { config } }

            pub fn spawn_body(&mut self, _id: EntityId, _props: &PhysicsProps) {}
            pub fn despawn_body(&mut self, _id: EntityId) {}
            pub fn update_spring_targets(&mut self, _targets: &HashMap<EntityId, LayoutRect>) {}
            pub fn apply_impulse(&mut self, _id: EntityId, _impulse: Vec2) {}

            pub fn step(&mut self) -> HashMap<EntityId, (Transform2D, Vec2)> {
                HashMap::new()
            }
        }
    """)),

    ("crates/fe_ui_rapier/src/solver.rs", textwrap.dedent("""\
        //! solver.rs — The Spring Constraint Solver.

        pub const SETTLE_THRESHOLD_PX:      f32 = 0.5;
        pub const SLEEP_THRESHOLD_PX:       f32 = 0.1;
        pub const SLEEP_VELOCITY_THRESHOLD: f32 = 1.0;

        // TODO: implement update_spring_targets, evaluate_sleep, wake_body
    """)),

    stub("crates/fe_ui_rapier/src/joints.rs",  "Fixed, Spring, Free joint implementations"),
    stub("crates/fe_ui_rapier/src/collider.rs","Collider shapes: rect, circle, squircle"),
    stub("crates/fe_ui_rapier/src/gesture.rs", "Gesture velocity → impulse injection into Rapier bodies"),
    stub("crates/fe_ui_rapier/src/portal.rs",  "Portal boundary colliders — rigid and spring types"),

    # ──────────────────────────────────────────────────────────────────────
    # crates/fe_ui_wgpu
    # ──────────────────────────────────────────────────────────────────────
    ("crates/fe_ui_wgpu/Cargo.toml", textwrap.dedent("""\
        [package]
        name        = "fe_ui_wgpu"
        version     = "0.1.0-alpha"
        edition     = "2021"
        description = "wgpu rendering pipeline for Ferrum fe_ui"
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

        use fe_ui_core::types::{Transform2D, Vec2};

        pub fn interpolate_transform(prev: &Transform2D, curr: &Transform2D, alpha: f32) -> Transform2D {
            Transform2D {
                position: prev.position.lerp(curr.position, alpha),
                rotation: prev.rotation + (curr.rotation - prev.rotation) * alpha,
                scale:    prev.scale.lerp(curr.scale, alpha),
            }
        }

        pub fn render_velocity(prev_pos: Vec2, curr_pos: Vec2, physics_hz: f32, pixels_per_meter: f32) -> Vec2 {
            (curr_pos - prev_pos) * physics_hz * pixels_per_meter
        }
    """)),

    ("crates/fe_ui_wgpu/src/instance.rs", textwrap.dedent("""\
        //! instance.rs — SDF instance buffer packing.

        use bytemuck::{Pod, Zeroable};

        #[repr(C)]
        #[derive(Debug, Clone, Copy, Pod, Zeroable)]
        pub struct SdfInstance {
            pub transform:     [[f32; 3]; 3],
            pub size:          [f32; 2],
            pub corner_radius: f32,
            pub shape_type:    u32,
            pub velocity:      [f32; 2],
            pub altitude:      f32,
            pub z_depth:       f32,
            pub color:         [f32; 4],
            pub border_color:  [f32; 4],
            pub border_width:  f32,
            pub glow_color:    [f32; 4],
            pub glow_radius:   f32,
            pub _pad:          [f32; 2],
        }

        // TODO: implement instance buffer, upload, and per-frame packing
    """)),

    ("crates/fe_ui_wgpu/src/atlas.rs", textwrap.dedent("""\
        //! atlas.rs — SDF glyph atlas.
        //! TODO: implement GlyphAtlas with LRU eviction
    """)),

    stub("crates/fe_ui_wgpu/src/renderer.rs",    "Top-level Renderer — owns wgpu device/surface, runs draw passes"),
    stub("crates/fe_ui_wgpu/src/pipeline.rs",    "wgpu render pipeline construction and shader compilation"),
    stub("crates/fe_ui_wgpu/src/portal.rs",      "Stencil mask rendering for Portal clipping boundaries"),
    stub("crates/fe_ui_wgpu/src/text/mod.rs",    "Text rendering module"),
    stub("crates/fe_ui_wgpu/src/text/shaper.rs", "cosmic-text shaping integration"),
    stub("crates/fe_ui_wgpu/src/text/layout.rs", "Text layout — wrapping, RTL, line breaking"),

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

        winit     = { workspace = true }
        log       = { workspace = true }
        accesskit = { workspace = true }
    """)),

    ("crates/fe_ui/src/lib.rs", textwrap.dedent("""\
        //! fe_ui — The Ferrum framework orchestrator.

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
        //! THE execution order defined here is sacred. Do not reorder without an RFC.
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
        //!  10.  Render (interpolate → pack SDF buffer → draw)

        // TODO: implement run_frame(app, layout, physics, renderer, signals)
    """)),

    ("crates/fe_ui/src/sync.rs", textwrap.dedent("""\
        //! sync.rs — The Sync Point.
        //! Signals are Intent. Physics is Reality. The Sync Point is where Intent becomes Reality.

        // TODO: implement SyncPoint::run(signals, layout, physics)
    """)),

    ("crates/fe_ui/src/macros.rs", textwrap.dedent("""\
        //! macros.rs — The #[component] macro and render! DSL.
        //! The only public door into the Ghost ECS.

        // TODO: implement proc-macro crate (fe_ui_macros) and re-export here.
    """)),

    stub("crates/fe_ui/src/app.rs",    "App builder — window config, subsystem init, run()"),
    stub("crates/fe_ui/src/window.rs", "Window configuration and winit integration"),
    stub("crates/fe_ui/src/event.rs",  "Input event routing — signals, impulses, gesture dispatch"),

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
    """)),

    ("crates/fe_std/src/lib.rs", textwrap.dedent("""\
        //! fe_std — Standard component library for Ferrum.

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
        //! Class: Kinematic-Spring (press deformation) + Dynamic-Impulse on click.

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
        //! Class: Kinematic-Spring. Critically damped — zero overshoot.
        // TODO: implement #[component] Switch
    """)),

    ("crates/fe_std/src/slider.rs", textwrap.dedent("""\
        //! slider.rs — The Slider component.
        //! Class: Kinematic-Spring. Thumb has mass — dragging feels weighted.
        // TODO: implement #[component] Slider
    """)),

    ("crates/fe_std/src/modal.rs", textwrap.dedent("""\
        //! modal.rs — The Modal component.
        //! Class: Dynamic-Impulse. Falls in with gravity, flick to dismiss.
        // TODO: implement #[component] Modal
    """)),

    ("crates/fe_std/src/scroll_area.rs", textwrap.dedent("""\
        //! scroll_area.rs — The ScrollArea component.
        //! Class: Dynamic-Impulse. Flick momentum, rubber-band over-scroll.
        // TODO: implement #[component] ScrollArea
    """)),

    stub("crates/fe_std/src/toast.rs",    "Toast — Dynamic-Impulse notification. Slides in, falls out."),
    stub("crates/fe_std/src/drawer.rs",   "Drawer — Dynamic-Impulse edge panel. Velocity-aware open/close."),
    stub("crates/fe_std/src/tab_bar.rs",  "TabBar — Kinematic-Spring. Indicator stretches during travel."),
    stub("crates/fe_std/src/checkbox.rs", "Checkbox — Kinematic-Spring. Check mark spring-draws on toggle."),
    stub("crates/fe_std/src/text.rs",     "Text — SDF-rendered, cosmic-text shaped, glyph-atlas backed."),

    # ──────────────────────────────────────────────────────────────────────
    # .ferrum/ (tooling cache — gitignored)
    # ──────────────────────────────────────────────────────────────────────
    (".ferrum/cache/glyphs/.gitkeep",  ""),
    (".ferrum/cache/shaders/.gitkeep", ""),
    (".ferrum/cache/assets.ron",       "// Asset manifest — generated by build.rs. Do not edit manually.\n"),

    # ──────────────────────────────────────────────────────────────────────
    # build.rs
    # ──────────────────────────────────────────────────────────────────────
    ("build.rs", textwrap.dedent("""\
        //! build.rs — Ferrum asset pipeline.

        fn main() {
            println!("cargo:rerun-if-changed=assets/");
            println!("cargo:rerun-if-changed=ferrum.toml");

            // TODO: implement shader validation, font bundling, typed asset handle generation
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

    created_files: list[Path] = []
    skipped_files: list[Path] = []

    for rel_path, content in FILES:
        target = root / rel_path
        target.parent.mkdir(parents=True, exist_ok=True)

        if target.exists():
            skipped_files.append(target)
            warn(f"EXISTS  {rel_path}")
            continue

        target.write_text(content, encoding="utf-8")
        created_files.append(target)
        ok(f"CREATE  {rel_path}")

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
