//! bouncing_button — The "Hello, World" of Ferrum.
//! Run: cargo run --example bouncing_button
//!
//! Demonstrates:
//!   - App + Window setup
//!   - A single RigidBody button with restitution
//!   - on_click impulse
//!   - use_signal for hover state

use fe_ui::prelude::*;

fn main() {
    App::new()
        .add_window(Window::default().title("Bouncing Button"))
        .run(ui);
}

#[component]
fn ui(cx: Scope) -> Element {
    let count     = use_signal(cx, || 0_u32);
    let is_hovered = use_signal(cx, || false);

    render! {
        Flex {
            direction: Column,
            gap: 16.0,
            padding: 32.0,
            physics_world: PhysicsWorld::default(),

            Text { size: 32.0, "Count: {count}" }

            Button {
                physics: Physics::dynamic()
                    .mass(1.0)
                    .restitution(0.7),
                on_mouseenter: move |_| is_hovered.set(true),
                on_mouseleave: move |_| is_hovered.set(false),
                on_click: move |e| {
                    count.set(*count + 1);
                    e.apply_impulse(Vec2::new(0.0, -400.0));
                },
                "Click me ({count})"
            }
        }
    }
}
