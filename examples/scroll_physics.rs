//! scroll_physics — Demonstrates ScrollArea with flick momentum
//! and rubber-band over-scroll.
//!
//! Run: cargo run --example scroll_physics

use fe_ui::prelude::*;

fn main() {
    App::new()
        .add_window(Window::default().title("Scroll Physics"))
        .run(ui);
}

#[component]
fn ui(cx: Scope) -> Element {
    render! {
        ScrollArea {
            direction: ScrollDirection::Vertical,
            physics: ScrollPhysics {
                friction: 0.88,
                overscroll: Overscroll::RubberBand {
                    stiffness: 180.0,
                    max_displacement: 80.0,
                },
            },
            // TODO: populate with list items
        }
    }
}
