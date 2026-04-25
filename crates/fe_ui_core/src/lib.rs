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
