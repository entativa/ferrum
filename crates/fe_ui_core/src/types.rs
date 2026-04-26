// crates/fe_ui_core/src/types.rs
//
// Shared primitive types used across all Ferrum crates.
//
// ─── Design rules ────────────────────────────────────────────────────────────
//
//  - Every type here is owned by fe_ui_core.
//  - No subsystem crate (taffy, rapier, wgpu) defines its own primitives —
//    they all speak these types.
//  - glam::Vec2 is re-exported directly so callers never need to import glam.
//  - No heavy dependencies. This file must compile in milliseconds.
//
// ─────────────────────────────────────────────────────────────────────────────

pub use glam::Vec2;

// ─── LayoutRect ──────────────────────────────────────────────────────────────

/// The output of Taffy layout. The input to Rapier spring targets.
/// The input to wgpu SDF quad sizing.
///
/// Coordinate system: origin top-left, x right, y down.
/// Units: logical pixels (device-independent).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayoutRect {
    pub x:      f32,
    pub y:      f32,
    pub width:  f32,
    pub height: f32,
}

impl LayoutRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Centre point of the rect. Used as the spring target position.
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Size as a Vec2. Used for SDF quad sizing.
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.width, self.height)
    }

    /// Top-left origin.
    pub fn origin(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Returns true if the point is inside the rect (inclusive).
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// Returns true if this rect overlaps another.
    pub fn intersects(&self, other: &LayoutRect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width  > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Expand the rect by `amount` on all sides.
    /// Used for frustum culling with motion blur padding.
    pub fn expand(&self, amount: f32) -> Self {
        Self {
            x:      self.x      - amount,
            y:      self.y      - amount,
            width:  self.width  + amount * 2.0,
            height: self.height + amount * 2.0,
        }
    }

    /// Linearly interpolate between two rects.
    pub fn lerp(&self, other: &LayoutRect, t: f32) -> Self {
        Self {
            x:      lerp_f32(self.x,      other.x,      t),
            y:      lerp_f32(self.y,      other.y,      t),
            width:  lerp_f32(self.width,  other.width,  t),
            height: lerp_f32(self.height, other.height, t),
        }
    }
}

// ─── Transform2D ─────────────────────────────────────────────────────────────

/// The output of Rapier physics simulation (after interpolation).
/// Consumed by fe_ui_wgpu to position and orient SDF quads.
///
/// Rotation is in radians. Scale is relative (1.0 = no scale).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub position: Vec2,
    pub rotation: f32,
    pub scale:    Vec2,
}

impl Transform2D {
    pub fn new(position: Vec2, rotation: f32, scale: Vec2) -> Self {
        Self { position, rotation, scale }
    }

    pub fn from_position(position: Vec2) -> Self {
        Self {
            position,
            rotation: 0.0,
            scale:    Vec2::ONE,
        }
    }

    /// Linearly interpolate position and scale, slerp rotation.
    ///
    /// `alpha` is in [0.0, 1.0]:
    ///   0.0 → fully `self` (previous physics step)
    ///   1.0 → fully `other` (current physics step)
    ///
    /// Uses slerp for rotation to always take the shortest angular path.
    /// Using lerp for rotation causes incorrect behaviour at high angular
    /// velocities (e.g. a spinning card wraps the wrong way).
    pub fn interpolate(&self, other: &Transform2D, alpha: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, alpha),
            rotation: slerp_angle(self.rotation, other.rotation, alpha),
            scale:    self.scale.lerp(other.scale, alpha),
        }
    }

    /// Convert to a column-major 3x3 matrix for the GPU instance buffer.
    /// Layout: [col0, col1, col2] where each col is [x, y, z].
    pub fn to_mat3(&self) -> [[f32; 3]; 3] {
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        let sx    = self.scale.x;
        let sy    = self.scale.y;
        let tx    = self.position.x;
        let ty    = self.position.y;

        [
            [cos_r * sx, sin_r * sx, 0.0],
            [-sin_r * sy, cos_r * sy, 0.0],
            [tx, ty, 1.0],
        ]
    }

    /// Identity transform — position zero, no rotation, scale 1.
    pub fn identity() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale:    Vec2::ONE,
        }
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::identity()
    }
}

// ─── Color ───────────────────────────────────────────────────────────────────

/// RGBA colour stored as f32 in [0.0, 1.0].
///
/// All Ferrum rendering uses linear colour space internally.
/// The wgpu pipeline applies sRGB conversion at output.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    // ── Constants ────────────────────────────────────────────────────────────

    pub const BLACK:       Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE:       Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const RED:         Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN:       Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE:        Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    // Ferrum brand purple
    pub const ACCENT: Color = Color {
        r: 0.486,  // #7c
        g: 0.227,  // #3a
        b: 0.929,  // #ed
        a: 1.0,
    };

    // ── Constructors ─────────────────────────────────────────────────────────

    /// Construct from individual f32 components in [0.0, 1.0].
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Construct from individual f32 components, fully opaque.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Construct from u8 components [0, 255].
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Parse a CSS hex colour string.
    ///
    /// Supported formats:
    ///   `#rrggbb`   — fully opaque
    ///   `#rrggbbaa` — with explicit alpha
    ///   `#rgb`      — shorthand, each nibble doubled (e.g. #f0a → #ff00aa)
    ///   `#rgba`     — shorthand with alpha
    ///
    /// The leading `#` is optional.
    ///
    /// # Panics (debug builds only)
    /// Panics with a descriptive message if the string is not a valid hex
    /// colour. In release builds, returns `Color::BLACK` instead.
    pub fn hex(s: &str) -> Self {
        match Self::try_hex(s) {
            Ok(c)  => c,
            Err(e) => {
                #[cfg(debug_assertions)]
                panic!("Color::hex(\"{s}\") — {e}");
                #[cfg(not(debug_assertions))]
                {
                    let _ = e;
                    Color::BLACK
                }
            }
        }
    }

    /// Parse a hex colour string, returning an error string on failure.
    /// Used internally by `hex()` and in tests.
    pub fn try_hex(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let s = s.strip_prefix('#').unwrap_or(s);

        match s.len() {
            // ── Shorthand: #rgb ───────────────────────────────────────────
            3 => {
                let r = expand_nibble(hex_nibble(s, 0)?);
                let g = expand_nibble(hex_nibble(s, 1)?);
                let b = expand_nibble(hex_nibble(s, 2)?);
                Ok(Self::rgba_u8(r, g, b, 255))
            }
            // ── Shorthand: #rgba ──────────────────────────────────────────
            4 => {
                let r = expand_nibble(hex_nibble(s, 0)?);
                let g = expand_nibble(hex_nibble(s, 1)?);
                let b = expand_nibble(hex_nibble(s, 2)?);
                let a = expand_nibble(hex_nibble(s, 3)?);
                Ok(Self::rgba_u8(r, g, b, a))
            }
            // ── Full: #rrggbb ─────────────────────────────────────────────
            6 => {
                let r = hex_byte(s, 0)?;
                let g = hex_byte(s, 2)?;
                let b = hex_byte(s, 4)?;
                Ok(Self::rgba_u8(r, g, b, 255))
            }
            // ── Full with alpha: #rrggbbaa ────────────────────────────────
            8 => {
                let r = hex_byte(s, 0)?;
                let g = hex_byte(s, 2)?;
                let b = hex_byte(s, 4)?;
                let a = hex_byte(s, 6)?;
                Ok(Self::rgba_u8(r, g, b, a))
            }
            n => Err(format!(
                "invalid hex colour length {n} (expected 3, 4, 6, or 8 hex digits after '#')"
            )),
        }
    }

    // ── Conversions ───────────────────────────────────────────────────────────

    /// Returns the colour as a [r, g, b, a] f32 array.
    /// Used for GPU uniform uploads.
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Returns the colour as u8 components [0, 255].
    pub fn to_rgba_u8(&self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }

    // ── Operations ───────────────────────────────────────────────────────────

    /// Return a copy with a different alpha value.
    pub fn with_alpha(&self, a: f32) -> Self {
        Self { a, ..*self }
    }

    /// Linearly interpolate between two colours.
    pub fn lerp(&self, other: &Color, t: f32) -> Self {
        Self {
            r: lerp_f32(self.r, other.r, t),
            g: lerp_f32(self.g, other.g, t),
            b: lerp_f32(self.b, other.b, t),
            a: lerp_f32(self.a, other.a, t),
        }
    }

    /// Premultiply alpha.
    /// All Ferrum render targets use premultiplied alpha — call this before
    /// uploading a colour as a GPU uniform if the target expects premultiplied.
    pub fn premultiply(&self) -> Self {
        Self {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }
}

// ─── Bounds ──────────────────────────────────────────────────────────────────

/// Axis-aligned bounding box (min/max corners).
/// Used for frustum culling in the render pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bounds {
    pub min: Vec2,
    pub max: Vec2,
}

impl Bounds {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Construct from a LayoutRect.
    pub fn from_rect(rect: &LayoutRect) -> Self {
        Self {
            min: Vec2::new(rect.x, rect.y),
            max: Vec2::new(rect.x + rect.width, rect.y + rect.height),
        }
    }

    pub fn width(&self)  -> f32 { self.max.x - self.min.x }
    pub fn height(&self) -> f32 { self.max.y - self.min.y }
    pub fn center(&self) -> Vec2 { (self.min + self.max) * 0.5 }

    /// Returns true if this bounds overlaps another.
    pub fn intersects(&self, other: &Bounds) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    /// Returns true if a point is inside the bounds.
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Expand bounds symmetrically by `amount` on all sides.
    /// Used to expand viewport bounds by MAX_BLUR_STRETCH before culling.
    pub fn expand(&self, amount: f32) -> Self {
        Self {
            min: self.min - Vec2::splat(amount),
            max: self.max + Vec2::splat(amount),
        }
    }
}

// ─── Math helpers ─────────────────────────────────────────────────────────────

/// Linear interpolation between two f32 values.
#[inline(always)]
pub fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Shortest-path angle interpolation (slerp for 2D rotation).
///
/// Always interpolates via the shortest angular distance.
/// Without this, a body rotating from 350° to 10° would incorrectly
/// interpolate 340° of the long way round instead of 20° of the short way.
#[inline]
pub fn slerp_angle(a: f32, b: f32, t: f32) -> f32 {
    use std::f32::consts::TAU;

    // Normalise both angles to [0, TAU)
    let a = a.rem_euclid(TAU);
    let b = b.rem_euclid(TAU);

    // Find the shortest delta
    let mut delta = b - a;
    if delta > std::f32::consts::PI  { delta -= TAU; }
    if delta < -std::f32::consts::PI { delta += TAU; }

    (a + delta * t).rem_euclid(TAU)
}

// ─── Hex parsing helpers ──────────────────────────────────────────────────────

/// Parse a single hex nibble (one char) from a string at a given char index.
fn hex_nibble(s: &str, char_index: usize) -> Result<u8, String> {
    let ch = s
        .chars()
        .nth(char_index)
        .ok_or_else(|| format!("index {char_index} out of bounds in \"{s}\""))?;

    ch.to_digit(16)
        .map(|d| d as u8)
        .ok_or_else(|| format!("'{ch}' is not a valid hex digit"))
}

/// Expand a single nibble to a full byte (e.g. 0xA → 0xAA).
fn expand_nibble(n: u8) -> u8 {
    n << 4 | n
}

/// Parse a hex byte (two chars) from a string starting at a given char index.
fn hex_byte(s: &str, char_index: usize) -> Result<u8, String> {
    let hi = hex_nibble(s, char_index)?;
    let lo = hex_nibble(s, char_index + 1)?;
    Ok(hi << 4 | lo)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    // ── Color::hex ────────────────────────────────────────────────────────────

    #[test]
    fn hex_rrggbb_opaque() {
        let c = Color::hex("#ff0000");
        assert_eq!(c.to_rgba_u8(), [255, 0, 0, 255]);
    }

    #[test]
    fn hex_rrggbb_no_hash() {
        let c = Color::hex("00ff00");
        assert_eq!(c.to_rgba_u8(), [0, 255, 0, 255]);
    }

    #[test]
    fn hex_rrggbbaa_with_alpha() {
        let c = Color::hex("#ffffff80");
        assert_eq!(c.to_rgba_u8(), [255, 255, 255, 128]);
    }

    #[test]
    fn hex_rrggbbaa_fully_transparent() {
        let c = Color::hex("#00000000");
        assert_eq!(c.to_rgba_u8(), [0, 0, 0, 0]);
    }

    #[test]
    fn hex_shorthand_rgb() {
        // #f00 → #ff0000
        let c = Color::hex("#f00");
        assert_eq!(c.to_rgba_u8(), [255, 0, 0, 255]);
    }

    #[test]
    fn hex_shorthand_rgba() {
        // #f00f → #ff0000ff
        let c = Color::hex("#f00f");
        assert_eq!(c.to_rgba_u8(), [255, 0, 0, 255]);
    }

    #[test]
    fn hex_shorthand_rgba_transparent() {
        // #0000 → fully transparent black
        let c = Color::hex("#0000");
        assert_eq!(c.to_rgba_u8(), [0, 0, 0, 0]);
    }

    #[test]
    fn hex_mixed_case() {
        let lower = Color::hex("#7c3aed");
        let upper = Color::hex("#7C3AED");
        assert_eq!(lower.to_rgba_u8(), upper.to_rgba_u8());
    }

    #[test]
    fn hex_whitespace_trimmed() {
        let c = Color::hex("  #ffffff  ");
        assert_eq!(c.to_rgba_u8(), [255, 255, 255, 255]);
    }

    #[test]
    fn hex_black() {
        let c = Color::hex("#000000");
        assert_eq!(c.to_rgba_u8(), [0, 0, 0, 255]);
    }

    #[test]
    fn hex_white() {
        let c = Color::hex("#ffffff");
        assert_eq!(c.to_rgba_u8(), [255, 255, 255, 255]);
    }

    #[test]
    fn hex_ferrum_accent() {
        // #7c3aed — Ferrum's brand purple
        let c = Color::hex("#7c3aed");
        let [r, g, b, a] = c.to_rgba_u8();
        assert_eq!(r, 0x7c);
        assert_eq!(g, 0x3a);
        assert_eq!(b, 0xed);
        assert_eq!(a, 255);
    }

    #[test]
    fn hex_try_hex_invalid_char() {
        assert!(Color::try_hex("#zzzzzz").is_err());
    }

    #[test]
    fn hex_try_hex_invalid_length() {
        assert!(Color::try_hex("#12345").is_err());   // 5 chars — invalid
        assert!(Color::try_hex("#1234567").is_err()); // 7 chars — invalid
        assert!(Color::try_hex("#").is_err());         // empty after #
    }

    #[test]
    fn hex_try_hex_valid_returns_ok() {
        assert!(Color::try_hex("#abc").is_ok());
        assert!(Color::try_hex("#abcd").is_ok());
        assert!(Color::try_hex("#aabbcc").is_ok());
        assert!(Color::try_hex("#aabbccdd").is_ok());
    }

    // ── Color operations ─────────────────────────────────────────────────────

    #[test]
    fn color_with_alpha() {
        let c = Color::RED.with_alpha(0.5);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.a, 0.5);
    }

    #[test]
    fn color_lerp_halfway() {
        let a = Color::BLACK;
        let b = Color::WHITE;
        let m = a.lerp(&b, 0.5);
        assert!((m.r - 0.5).abs() < 1e-6);
        assert!((m.g - 0.5).abs() < 1e-6);
        assert!((m.b - 0.5).abs() < 1e-6);
        assert!((m.a - 1.0).abs() < 1e-6);
    }

    #[test]
    fn color_lerp_at_zero_is_self() {
        let a = Color::RED;
        let b = Color::BLUE;
        let r = a.lerp(&b, 0.0);
        assert_eq!(r.to_array(), a.to_array());
    }

    #[test]
    fn color_lerp_at_one_is_other() {
        let a = Color::RED;
        let b = Color::BLUE;
        let r = a.lerp(&b, 1.0);
        assert_eq!(r.to_array(), b.to_array());
    }

    #[test]
    fn color_premultiply() {
        let c = Color::rgba(1.0, 0.5, 0.25, 0.5);
        let p = c.premultiply();
        assert!((p.r - 0.5).abs()  < 1e-6);
        assert!((p.g - 0.25).abs() < 1e-6);
        assert!((p.b - 0.125).abs() < 1e-6);
        assert!((p.a - 0.5).abs()  < 1e-6);
    }

    #[test]
    fn color_to_array_matches_fields() {
        let c = Color::rgba(0.1, 0.2, 0.3, 0.4);
        assert_eq!(c.to_array(), [0.1, 0.2, 0.3, 0.4]);
    }

    // ── LayoutRect ───────────────────────────────────────────────────────────

    #[test]
    fn layout_rect_center() {
        let r = LayoutRect::new(0.0, 0.0, 200.0, 100.0);
        assert_eq!(r.center(), Vec2::new(100.0, 50.0));
    }

    #[test]
    fn layout_rect_center_offset() {
        let r = LayoutRect::new(50.0, 20.0, 200.0, 100.0);
        assert_eq!(r.center(), Vec2::new(150.0, 70.0));
    }

    #[test]
    fn layout_rect_size() {
        let r = LayoutRect::new(0.0, 0.0, 300.0, 150.0);
        assert_eq!(r.size(), Vec2::new(300.0, 150.0));
    }

    #[test]
    fn layout_rect_contains_inside() {
        let r = LayoutRect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(Vec2::new(50.0, 50.0)));
    }

    #[test]
    fn layout_rect_contains_edge() {
        let r = LayoutRect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(Vec2::new(0.0, 0.0)));
        assert!(r.contains(Vec2::new(100.0, 100.0)));
    }

    #[test]
    fn layout_rect_contains_outside() {
        let r = LayoutRect::new(0.0, 0.0, 100.0, 100.0);
        assert!(!r.contains(Vec2::new(101.0, 50.0)));
        assert!(!r.contains(Vec2::new(50.0, -1.0)));
    }

    #[test]
    fn layout_rect_intersects_overlapping() {
        let a = LayoutRect::new(0.0,  0.0, 100.0, 100.0);
        let b = LayoutRect::new(50.0, 50.0, 100.0, 100.0);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    #[test]
    fn layout_rect_intersects_touching_edge_not_overlapping() {
        // Touching edges are NOT considered intersecting
        let a = LayoutRect::new(0.0,  0.0, 100.0, 100.0);
        let b = LayoutRect::new(100.0, 0.0, 100.0, 100.0);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn layout_rect_expand() {
        let r = LayoutRect::new(10.0, 10.0, 100.0, 100.0);
        let e = r.expand(5.0);
        assert_eq!(e.x,      5.0);
        assert_eq!(e.y,      5.0);
        assert_eq!(e.width,  110.0);
        assert_eq!(e.height, 110.0);
    }

    #[test]
    fn layout_rect_lerp() {
        let a = LayoutRect::new(0.0, 0.0, 100.0, 100.0);
        let b = LayoutRect::new(100.0, 100.0, 200.0, 200.0);
        let m = a.lerp(&b, 0.5);
        assert!((m.x      - 50.0).abs() < 1e-5);
        assert!((m.y      - 50.0).abs() < 1e-5);
        assert!((m.width  - 150.0).abs() < 1e-5);
        assert!((m.height - 150.0).abs() < 1e-5);
    }

    // ── Transform2D ──────────────────────────────────────────────────────────

    #[test]
    fn transform_default_is_identity() {
        let t = Transform2D::default();
        assert_eq!(t.position, Vec2::ZERO);
        assert_eq!(t.rotation, 0.0);
        assert_eq!(t.scale,    Vec2::ONE);
    }

    #[test]
    fn transform_interpolate_position() {
        let a = Transform2D::from_position(Vec2::new(0.0, 0.0));
        let b = Transform2D::from_position(Vec2::new(100.0, 0.0));
        let m = a.interpolate(&b, 0.5);
        assert!((m.position.x - 50.0).abs() < 1e-5);
    }

    #[test]
    fn transform_interpolate_at_zero_is_self() {
        let a = Transform2D::from_position(Vec2::new(10.0, 20.0));
        let b = Transform2D::from_position(Vec2::new(100.0, 200.0));
        let r = a.interpolate(&b, 0.0);
        assert!((r.position.x - 10.0).abs() < 1e-5);
        assert!((r.position.y - 20.0).abs() < 1e-5);
    }

    #[test]
    fn transform_interpolate_at_one_is_other() {
        let a = Transform2D::from_position(Vec2::new(0.0, 0.0));
        let b = Transform2D::from_position(Vec2::new(100.0, 200.0));
        let r = a.interpolate(&b, 1.0);
        assert!((r.position.x - 100.0).abs() < 1e-5);
        assert!((r.position.y - 200.0).abs() < 1e-5);
    }

    #[test]
    fn transform_to_mat3_identity() {
        let t = Transform2D::identity();
        let m = t.to_mat3();
        // Column 0: [cos(0)*1, sin(0)*1, 0] = [1, 0, 0]
        assert!((m[0][0] - 1.0).abs() < 1e-6);
        assert!((m[0][1] - 0.0).abs() < 1e-6);
        // Column 2: [tx, ty, 1] = [0, 0, 1]
        assert!((m[2][0] - 0.0).abs() < 1e-6);
        assert!((m[2][1] - 0.0).abs() < 1e-6);
        assert!((m[2][2] - 1.0).abs() < 1e-6);
    }

    // ── slerp_angle ──────────────────────────────────────────────────────────

    #[test]
    fn slerp_angle_halfway() {
        let r = slerp_angle(0.0, PI, 0.5);
        assert!((r - PI * 0.5).abs() < 1e-5);
    }

    #[test]
    fn slerp_angle_at_zero_is_start() {
        let r = slerp_angle(1.0, 2.0, 0.0);
        assert!((r - 1.0).abs() < 1e-5);
    }

    #[test]
    fn slerp_angle_at_one_is_end() {
        let r = slerp_angle(1.0, 2.0, 1.0);
        assert!((r - 2.0).abs() < 1e-5);
    }

    #[test]
    fn slerp_angle_takes_shortest_path() {
        // From 350° to 10° — shortest path is +20°, NOT -340°
        let from = 350.0_f32.to_radians();
        let to   =  10.0_f32.to_radians();
        let mid  = slerp_angle(from, to, 0.5);

        // Midpoint should be ~0° (= 360°), NOT ~180°
        let mid_deg = mid.to_degrees();
        // 0° and 360° are the same angle — check it's close to 0/360
        assert!(
            mid_deg < 5.0 || mid_deg > 355.0,
            "expected midpoint near 0°/360°, got {mid_deg}°"
        );
    }

    #[test]
    fn slerp_angle_no_wrap_on_normal_range() {
        // 10° to 90° — no wrap, midpoint should be ~50°
        let from = 10.0_f32.to_radians();
        let to   = 90.0_f32.to_radians();
        let mid  = slerp_angle(from, to, 0.5);
        let mid_deg = mid.to_degrees();
        assert!((mid_deg - 50.0).abs() < 1.0, "expected ~50°, got {mid_deg}°");
    }

    // ── Bounds ───────────────────────────────────────────────────────────────

    #[test]
    fn bounds_from_rect() {
        let r = LayoutRect::new(10.0, 20.0, 100.0, 50.0);
        let b = Bounds::from_rect(&r);
        assert_eq!(b.min, Vec2::new(10.0, 20.0));
        assert_eq!(b.max, Vec2::new(110.0, 70.0));
    }

    #[test]
    fn bounds_intersects() {
        let a = Bounds::new(Vec2::new(0.0, 0.0),   Vec2::new(100.0, 100.0));
        let b = Bounds::new(Vec2::new(50.0, 50.0), Vec2::new(150.0, 150.0));
        assert!(a.intersects(&b));
    }

    #[test]
    fn bounds_no_intersect() {
        let a = Bounds::new(Vec2::new(0.0,   0.0),   Vec2::new(100.0, 100.0));
        let b = Bounds::new(Vec2::new(200.0, 200.0), Vec2::new(300.0, 300.0));
        assert!(!a.intersects(&b));
    }

    #[test]
    fn bounds_expand() {
        let b = Bounds::new(Vec2::new(10.0, 10.0), Vec2::new(90.0, 90.0));
        let e = b.expand(10.0);
        assert_eq!(e.min, Vec2::new(0.0,   0.0));
        assert_eq!(e.max, Vec2::new(100.0, 100.0));
    }

    // ── lerp_f32 ─────────────────────────────────────────────────────────────

    #[test]
    fn lerp_f32_halfway() {
        assert!((lerp_f32(0.0, 10.0, 0.5) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_f32_at_zero() {
        assert!((lerp_f32(3.0, 7.0, 0.0) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_f32_at_one() {
        assert!((lerp_f32(3.0, 7.0, 1.0) - 7.0).abs() < 1e-6);
    }
}
