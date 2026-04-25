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
