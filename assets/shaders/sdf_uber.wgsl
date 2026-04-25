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
