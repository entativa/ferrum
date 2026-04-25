//! fe_std_showcase — All standard components in one scene.
//! Use this as a visual regression reference.
//!
//! Run: cargo run --example fe_std_showcase

use fe_ui::prelude::*;
use fe_std::prelude::*;

fn main() {
    App::new()
        .add_window(Window::default().title("fe_std Showcase"))
        .run(ui);
}

#[component]
fn ui(cx: Scope) -> Element {
    // TODO: populate with all fe_std components
    render! { Flex { direction: Column } }
}
