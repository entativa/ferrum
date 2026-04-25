//! fe_ui — The Ferrum framework orchestrator.

pub mod app;
pub mod event;
pub mod frame;
pub mod macros;
pub mod sync;
pub mod window;

pub mod prelude {
    pub use crate::app::App;
    pub use crate::window::Window;
    pub use fe_ui_core::{
        entity::EntityId,
        physics::{PhysicsProps, PhysicsState, Stiffness, BodyType, InteractionClass},
        signal::{Signal, LayoutSignal, use_signal, use_layout_signal, use_derived},
        style::{Style, Dimension, FlexDirection, AlignItems, JustifyContent},
        types::{Color, LayoutRect, Transform2D, Vec2, Bounds},
    };
}
