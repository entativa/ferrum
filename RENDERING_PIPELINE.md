# Ferrum `fe_ui` — The Iron-Sight Rendering Pipeline

> Traditional UI draws textures. Ferrum draws mathematics. Every primitive is a shader. Every shadow is derived from physics. Every blur is earned by velocity.

---

## Overview

The Iron-Sight pipeline has four responsibilities, in order:

1. **Interpolate** — resolve smooth per-frame transforms from 240Hz physics data
2. **Cull & Sort** — depth-sort the scene, discard off-screen bodies
3. **Batch** — pack all standard primitives into a single instance buffer
4. **Draw** — one SDF uber-shader pass per material class

---

## Stage 1 — Physics Interpolation

The physics world steps at 240Hz. The renderer draws at 60/120/144Hz. These rates never align. Without interpolation, fast-moving widgets stutter — you see the physics step granularity as discrete jumps.

### The Alpha

Every render frame, compute an interpolation alpha from the time elapsed since the last physics step:

```rust
// How far between the last physics step and the next one are we right now?
let alpha: f32 = (frame_time - last_physics_step_time) / physics_step_duration;

// Interpolate position
let render_pos = prev_pos.lerp(curr_pos, alpha);

// Interpolate rotation (use slerp for correctness under high angular velocity)
let render_rot = prev_rot.slerp(curr_rot, alpha);
```

Get `alpha` wrong and fast motion produces a half-frame visual lag that *feels* like input latency even when measured latency is fine. This is the most common source of "something feels off" in physics UI.

### Velocity Extraction

After interpolation, extract the per-widget render velocity for motion blur:

```rust
// Pixels per second at render scale
let render_velocity: Vec2 = (curr_pos - prev_pos) * physics_hz * pixels_per_meter;
```

This velocity vector is passed directly into the vertex shader as `v_velocity`. It drives Stage 4.

---

## Stage 2 — Cull & Depth Sort

### Frustum Culling

Before touching the GPU, discard widgets whose interpolated AABB does not intersect the viewport. Physics bodies exist outside the viewport during high-velocity throws — they still simulate, they just don't draw.

```rust
// Expanded by max motion blur stretch to avoid popping at screen edges
let cull_rect = viewport.expand(MAX_BLUR_STRETCH_PX);
bodies.retain(|b| b.aabb().intersects(cull_rect));
```

### Z-Depth Assignment

Even though Ferrum is a 2D UI framework, every widget gets a Z coordinate. Z is derived from two sources:

| Source | Contribution |
|---|---|
| **Scene hierarchy depth** | Base Z — parent always behind child |
| **`altitude` property** | Offset Z — higher altitude floats above siblings |

```rust
let z: f32 = (hierarchy_depth as f32 * Z_LAYER_STEP)
           + (widget.altitude * Z_ALTITUDE_SCALE);
```

Z-depth is written to the depth buffer. This prevents Z-fighting when two physics bodies collide and overlap — the hierarchy always wins, altitude breaks ties between siblings. No painter's algorithm. No manual z-index wrestling.

### Shadow Depth

Shadow offset and blur radius are derived from altitude, not declared:

```rust
// In the SDF shader — no shadow config needed from the developer
let shadow_offset = vec2(altitude * 0.8, altitude * 1.2); // px
let shadow_blur   = altitude * 2.5;                        // px
let shadow_alpha  = clamp(altitude / MAX_ALTITUDE, 0.0, 0.6);
```

A widget at `altitude: 0.0` has no shadow. A widget at `altitude: 8.0` casts a deep, soft shadow. Physical height becomes visual depth automatically.

---

## Stage 3 — GPU-Side Batching

### The Instance Buffer

All standard SDF primitives share one vertex buffer (a unit quad) and one instance buffer. One draw call covers the entire UI.

```rust
// Per-instance data — packed into the instance buffer
#[repr(C)]
struct SdfInstance {
    transform:   Mat3,    // position, rotation, scale from physics interpolation
    size:        Vec2,    // width, height in pixels
    corner_radius: f32,   // 0.0 = rect, > 0.0 = rounded rect, size/2 = circle
    shape_type:  u32,     // 0 = rect, 1 = circle, 2 = squircle, 3 = text_quad
    velocity:    Vec2,    // for motion blur
    altitude:    f32,     // for shadow derivation
    z_depth:     f32,     // depth buffer value
    color:       Vec4,    // base fill color
    border_color: Vec4,
    border_width: f32,
    gradient:    GradientUniform, // type + two stops (zero cost if unused)
    glow_color:  Vec4,
    glow_radius: f32,
}
```

The GPU iterates this buffer. One draw call. ~1μs CPU overhead regardless of widget count.

### Batching Tiers

Not everything fits the standard batch. Ferrum uses three tiers:

| Tier | Contents | Draw calls |
|---|---|---|
| **Tier 0 — SDF Batch** | All standard primitives (rect, circle, squircle, text quads) | 1 |
| **Tier 1 — Custom Material** | Widgets with a custom `.wgsl` shader | 1 per unique shader |
| **Tier 2 — Portal Mask** | Clipped overflow widgets (see Portal section) | 1 per portal boundary |

Custom material widgets (e.g., `glow_button.wgsl`) break the Tier 0 batch. They are sorted to the end of the draw list and drawn in a separate instanced call per unique shader. A standard app with 0 custom materials pays zero cost for this system.

**Rule**: Tier 0 is the fast path. Tier 1 is the escape hatch. Build `fe_std` exclusively on Tier 0.

---

## Stage 4 — The SDF Uber-Shader

One shader handles all standard primitives. Operations execute in a fixed order — changing the order changes the visual result.

### Operation Order (Load-Bearing)

```
1. Velocity Stretch   → warp UV domain along velocity vector (motion blur setup)
2. SDF Evaluation     → compute signed distance at the (warped) UV coordinate
3. Shadow Sample      → sample SDF expanded outward by blur radius (outside shape)
4. Fill               → evaluate gradient or solid color inside the shape boundary
5. Border             → evaluate color at the SDF edge band
6. Glow               → additive bloom outside the border
7. Motion Blur Blend  → blend original and velocity-stretched samples by speed
8. Alpha Composite    → premultiplied alpha output
```

Swapping any two steps produces incorrect output. Shadow must sample *before* fill so it composites under the shape. Border must come after fill so it renders on top. Glow is additive over everything except the motion blur blend.

### The Shader

```wgsl
// fe_ui/crates/fe_ui_wgpu/src/shaders/sdf_uber.wgsl

struct SdfInstance {
    transform:     mat3x3<f32>,
    size:          vec2<f32>,
    corner_radius: f32,
    shape_type:    u32,
    velocity:      vec2<f32>,
    altitude:      f32,
    color:         vec4<f32>,
    border_color:  vec4<f32>,
    border_width:  f32,
    glow_color:    vec4<f32>,
    glow_radius:   f32,
    // ... gradient uniforms
}

// Signed distance for a rounded rectangle
fn sdf_rounded_rect(p: vec2<f32>, size: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - size * 0.5 + r;
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let inst = instances[in.instance_index];

    // 1. Velocity stretch — warp UV along motion vector
    let speed       = length(inst.velocity);
    let blur_uv     = in.local_uv + normalize(inst.velocity) * BLUR_SCALE * speed;
    let sample_uv   = mix(in.local_uv, blur_uv, clamp(speed / BLUR_THRESHOLD, 0.0, 1.0));

    // 2. SDF evaluation at warped UV
    let p   = (sample_uv - 0.5) * inst.size;
    let sdf = sdf_rounded_rect(p, inst.size, inst.corner_radius);

    // 3. Shadow (samples SDF at expanded distance — outside the shape)
    let shadow_blur   = inst.altitude * 2.5;
    let shadow_alpha  = clamp(inst.altitude / MAX_ALTITUDE, 0.0, 0.6);
    let shadow_sdf    = sdf + shadow_blur;
    let shadow        = vec4(0.0, 0.0, 0.0, shadow_alpha)
                      * smoothstep(shadow_blur, 0.0, shadow_sdf);

    // 4. Fill (gradient or solid — inside shape)
    let fill_alpha = smoothstep(1.0, -1.0, sdf);          // anti-aliased edge
    let fill       = vec4(inst.color.rgb, fill_alpha * inst.color.a);

    // 5. Border (at the SDF edge band)
    let border_inner = sdf + inst.border_width;
    let border_alpha = smoothstep(-1.0, 1.0, sdf)         // outer edge
                     * smoothstep(-1.0, 1.0, -border_inner); // inner edge
    let border       = vec4(inst.border_color.rgb, border_alpha);

    // 6. Glow (additive bloom outside border)
    let glow_sdf   = sdf - inst.border_width;
    let glow_alpha = clamp(-glow_sdf / inst.glow_radius, 0.0, 1.0);
    let glow       = inst.glow_color * glow_alpha * glow_alpha; // quadratic falloff

    // 7. Composite — shadow under fill, border over fill, glow additive
    var out = shadow;
    out = mix(out, fill,   fill.a);
    out = mix(out, border, border.a);
    out += glow * (1.0 - fill.a); // glow only outside the fill boundary

    // 8. Premultiplied alpha output
    return vec4(out.rgb * out.a, out.a);
}
```

### Why Premultiplied Alpha

Standard alpha blending causes dark fringing on anti-aliased SDF edges when compositing over colored backgrounds. Premultiplied alpha eliminates this — the edge pixels carry their contribution already baked in. All Ferrum render targets use premultiplied alpha throughout.

---

## Stage 4b — Velocity-Based Motion Blur

Velocity blur is opt-in at the scene level. Off by default — zero cost when disabled.

```rust
// scene.rs
world.set_renderer(RenderConfig {
    motion_blur: MotionBlur::Velocity {
        threshold:  200.0,  // px/s — below this, no blur applied
        max_stretch: 24.0,  // px — maximum UV stretch distance
        samples:    4,      // quality vs. performance tradeoff
    },
    // or:
    motion_blur: MotionBlur::Off,
});
```

At `threshold: 200.0`, a button moving at 800px/s gets full blur. A button animating at 50px/s gets none. The blur scales linearly between threshold and max velocity.

**The visual result**: Fast-moving widgets feel filmic. A notification sliding in from off-screen arrives with intent, not with strobing. A dismissed card exits with weight.

---

## The Portal — Clipping in a Physics World

When a widget has real momentum, it *will* overshoot its parent container. Standard clipping (scissor rect) just cuts the widget at the boundary — the physical illusion breaks. The widget teleports back inside.

Ferrum handles this with **The Portal**.

### How It Works

A `ScrollArea` or `ClipBounds` container defines a portal boundary. Widgets inside it are rendered using a **stencil mask** rather than a scissor rect.

```
Standard clip:  widget is cut at the boundary. Hard edge. Physically wrong.

Portal clip:    widget renders fully. A stencil mask derived from the container
                SDF is applied. The widget "exists" beyond the boundary but
                is not visible beyond it. The physics body still simulates
                outside the visible area.
```

The critical difference: the **physics body continues to simulate** past the portal boundary. It bounces off the container walls (which are Rapier colliders) and comes back. The portal just controls what the player sees — like a game camera.

```rust
ScrollArea {
    // Portal boundary = Rapier collider + stencil mask
    portal: Portal::bounded()
        .collider(true)          // physics bodies bounce off edges
        .overflow(Overflow::Clip), // visually hidden beyond boundary

    // Child widgets simulate freely, visible only inside the portal
    PlayerCard { }
    PlayerCard { }
    PlayerCard { }
}
```

### The Rubber-Band as Portal Physics

Over-scroll is the portal boundary's collider acting as a **soft constraint** rather than a rigid wall:

```rust
portal: Portal::bounded()
    .collider_type(Collider::Spring {
        stiffness: 180.0,
        max_displacement: 80.0, // px before hard stop
    })
```

The child body enters the spring zone, decelerates, and returns. The stencil mask stays fixed at the hard boundary. Visually: the content stretches slightly beyond the edge and snaps back — identical to iOS scroll behavior, produced by real physics.

---

## Rendering Lifecycle — Full Frame

```
FRAME START
│
├─ [Physics Interpolation]
│    alpha = (frame_time - last_step_time) / step_duration
│    render_pos  = lerp(prev_pos,  curr_pos,  alpha)
│    render_rot  = slerp(prev_rot, curr_rot,  alpha)
│    render_vel  = (curr_pos - prev_pos) × physics_hz × ppm
│
├─ [Cull & Sort]
│    frustum cull (expanded by MAX_BLUR_STRETCH)
│    depth sort by Z (hierarchy + altitude)
│    classify into Tier 0 / Tier 1 / Tier 2
│
├─ [Instance Buffer Update]
│    write SdfInstance per Tier 0 widget (CPU → GPU, one upload)
│    bind custom material uniforms for Tier 1 widgets
│    update portal stencil masks for Tier 2 widgets
│
├─ [Draw]
│    Pass 1 — Portal stencil write (Tier 2 boundaries)
│    Pass 2 — Tier 0 instanced draw (1 draw call, all standard primitives)
│    Pass 3 — Tier 1 draws (1 per unique custom shader, instanced)
│    Pass 4 — Composite + tonemap
│
FRAME END
```

---

## Performance Targets

| Metric | Target | Notes |
|---|---|---|
| Draw calls (standard UI) | 1–3 | 1 SDF batch + portal passes |
| CPU frame budget | < 1ms | Instance buffer write + cull |
| GPU frame budget | < 4ms @ 1440p | SDF evaluation is cheap per pixel |
| Motion blur overhead | < 0.5ms | Only on widgets above velocity threshold |
| Text glyph atlas miss | < 1% | LRU eviction tuned to common glyph sets |

---

## What This Enables for `fe_std`

Every `fe_std` component is built exclusively on Tier 0. That means:

- **`Button`**: rounded rect SDF, border, glow on hover, motion blur on throw
- **`Modal`**: altitude-derived shadow, portal boundary, spring drop-in
- **`ScrollArea`**: portal with rubber-band collider, child motion blur on flick
- **`Slider`**: circle SDF thumb with restitution, rect SDF track
- **`Switch`**: squircle SDF track, circle SDF thumb, spring snap

All of these are one draw call combined. The entire `fe_std` library costs the GPU the same as drawing a single textured quad in a traditional framework.

---

*See also: [Scene & Component Spec](./SCENE_SPEC.md) — physics configuration that feeds the interpolation stage.*
*See also: [Signal Graph](./SIGNAL_GRAPH.md) — how layout targets reach the instance buffer.*
