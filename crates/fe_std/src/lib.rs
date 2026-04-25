//! fe_std — Standard component library for Ferrum.

pub mod button;
pub mod checkbox;
pub mod drawer;
pub mod modal;
pub mod scroll_area;
pub mod slider;
pub mod switch;
pub mod tab_bar;
pub mod text;
pub mod toast;

pub mod prelude {
    pub use crate::{
        button::Button,
        checkbox::Checkbox,
        drawer::Drawer,
        modal::Modal,
        scroll_area::ScrollArea,
        slider::Slider,
        switch::Switch,
        tab_bar::TabBar,
        text::Text,
        toast::Toast,
    };
}
