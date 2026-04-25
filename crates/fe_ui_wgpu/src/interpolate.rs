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
