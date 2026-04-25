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
