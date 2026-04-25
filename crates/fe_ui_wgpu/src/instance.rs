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
