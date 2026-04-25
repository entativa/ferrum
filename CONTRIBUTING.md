# Contributing to Ferrum

## Getting Started

```bash
git clone https://github.com/ferrum-ui/fe_ui
cd fe_ui
cargo build
cargo test --workspace
```

## Contribution Rules

1. No cross-subsystem imports — subsystem crates speak `fe_ui_core` only.
2. No public `EntityId` — never exposed in a public API.
3. No `GhostWorld` access outside macros.
4. Frame loop order in `frame.rs` is sacred — RFC required to change.
5. `fe_ui_core` must compile in < 3s — keep it lean.
6. `fe_std` uses only the public API.

## Where to Start

- Phase 1: settle/sleep logic — `crates/fe_ui_rapier/src/solver.rs`
- Phase 1: SDF glyph atlas  — `crates/fe_ui_wgpu/src/text/atlas.rs`
- Phase 2: gesture tracking — `crates/fe_ui_core/src/input/gesture.rs`

See [ARCHITECTURE.md](./ARCHITECTURE.md) before touching any crate internals.
