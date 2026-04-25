//! physics_debug — Runs with show_physics_debug forced on.
//! Shows Taffy rects, Rapier colliders, sleep states, and
//! spring force vectors as overlays.
//!
//! Run: cargo run --example physics_debug

use fe_ui::prelude::*;

fn main() {
    App::new()
        .add_window(Window::default().title("Physics Debug"))
        .configure_renderer(|r| r.show_physics_debug(true))
        .run(ui);
}

#[component]
fn ui(cx: Scope) -> Element {
    render! {
        // TODO: populate with a representative scene
        Flex { direction: Column }
    }
}
