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
