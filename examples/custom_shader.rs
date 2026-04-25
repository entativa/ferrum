//! custom_shader — Demonstrates a Tier-1 custom .wgsl material
//! applied to a CustomBox widget. Note: breaks the SDF batch.
//!
//! Run: cargo run --example custom_shader

use fe_ui::prelude::*;

fn main() {
    App::new()
        .add_window(Window::default().title("Custom Shader"))
        .run(ui);
}

#[component]
fn ui(cx: Scope) -> Element {
    render! {
        CustomBox {
            shader: "assets/shaders/glow_button.wgsl",
            style: style! { width: 200.px, height: 60.px },
        }
    }
}
