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
#[derive(Debug, Clone, Copy, PartialEq)]
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
