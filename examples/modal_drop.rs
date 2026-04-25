//! modal_drop — Demonstrates the Modal spatial component.
//! A modal drops from above with gravity and catches on a spring.
//! Flick to dismiss.
//!
//! Run: cargo run --example modal_drop

use fe_ui::prelude::*;

fn main() {
    App::new()
        .add_window(Window::default().title("Modal Drop"))
        .run(ui);
}

#[component]
fn ui(cx: Scope) -> Element {
    let show = use_signal(cx, || false);

    render! {
        Flex {
            direction: Column,
            align_items: Center,
            justify_content: Center,
            physics_world: PhysicsWorld::default(),

            Button {
                on_click: move |_| show.set(true),
                "Open Modal"
            }

            Modal {
                visible: show,
                on_dismiss: move |_| show.set(false),
                ModalBody { Text { "Hello from a physical modal!" } }
            }
        }
    }
}
