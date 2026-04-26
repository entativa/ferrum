// crates/fe_ui_taffy/src/convert.rs
//
// fe_ui_core::Style → taffy::Style conversion.
// Targets taffy 0.4.4 API.
//
// ─── Key mapping decisions ────────────────────────────────────────────────────
//
//  Dimension::Percent(f32)
//    fe_ui_core stores [0,100] (CSS convention: 50.0 = "50%")
//    taffy expects  [0,1]   (0.5 = "50%")
//    → divide by 100.0
//
//  Gap
//    fe_ui_core: single f32 applied to both axes
//    taffy: Size<LengthPercentage> (row gap, column gap)
//    → apply to both width and height
//
// ─────────────────────────────────────────────────────────────────────────────

use fe_ui_core::style::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, Rect, Style,
};
use taffy::prelude::*;

/// Convert a `fe_ui_core::Style` to a `taffy::Style`.
pub fn to_taffy_style(style: &Style) -> taffy::Style {
    taffy::Style {
        display:         to_taffy_display(style.display),
        flex_direction:  to_taffy_flex_direction(style.flex_direction),
        flex_grow:       style.flex_grow,
        flex_shrink:     style.flex_shrink,
        align_items:     Some(to_taffy_align_items(style.align_items)),
        justify_content: Some(to_taffy_justify_content(style.justify_content)),
        size: Size {
            width:  to_taffy_dimension(style.width),
            height: to_taffy_dimension(style.height),
        },
        min_size: Size {
            width:  to_taffy_dimension(style.min_width),
            height: to_taffy_dimension(style.min_height),
        },
        max_size: Size {
            width:  to_taffy_dimension(style.max_width),
            height: to_taffy_dimension(style.max_height),
        },
        padding: to_taffy_rect_lp(style.padding),
        margin:  to_taffy_rect_lpa(style.margin),
        gap: Size {
            width:  LengthPercentage::Length(style.gap),
            height: LengthPercentage::Length(style.gap),
        },
        ..taffy::Style::DEFAULT
    }
}

// ─── Individual converters ────────────────────────────────────────────────────

fn to_taffy_display(d: Display) -> taffy::Display {
    match d {
        Display::Flex  => taffy::Display::Flex,
        Display::Grid  => taffy::Display::Grid,
        Display::Block => taffy::Display::Block,
        Display::None  => taffy::Display::None,
    }
}

fn to_taffy_flex_direction(d: FlexDirection) -> taffy::FlexDirection {
    match d {
        FlexDirection::Row           => taffy::FlexDirection::Row,
        FlexDirection::Column        => taffy::FlexDirection::Column,
        FlexDirection::RowReverse    => taffy::FlexDirection::RowReverse,
        FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
    }
}

fn to_taffy_align_items(a: AlignItems) -> taffy::AlignItems {
    match a {
        AlignItems::Stretch   => taffy::AlignItems::Stretch,
        AlignItems::Center    => taffy::AlignItems::Center,
        AlignItems::FlexStart => taffy::AlignItems::FlexStart,
        AlignItems::FlexEnd   => taffy::AlignItems::FlexEnd,
        AlignItems::Baseline  => taffy::AlignItems::Baseline,
    }
}

fn to_taffy_justify_content(j: JustifyContent) -> taffy::JustifyContent {
    match j {
        JustifyContent::FlexStart    => taffy::JustifyContent::FlexStart,
        JustifyContent::Center       => taffy::JustifyContent::Center,
        JustifyContent::FlexEnd      => taffy::JustifyContent::FlexEnd,
        JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
        JustifyContent::SpaceAround  => taffy::JustifyContent::SpaceAround,
        JustifyContent::SpaceEvenly  => taffy::JustifyContent::SpaceEvenly,
    }
}

/// Convert a `fe_ui_core::Dimension` to a `taffy::Dimension`.
/// Percent values divided by 100 — fe_ui_core stores [0,100], taffy wants [0,1].
pub fn to_taffy_dimension(d: Dimension) -> taffy::Dimension {
    match d {
        Dimension::Auto       => taffy::Dimension::Auto,
        Dimension::Px(px)     => taffy::Dimension::Length(px),
        Dimension::Percent(p) => taffy::Dimension::Percent(p / 100.0),
    }
}

fn to_taffy_rect_lp(r: Rect) -> taffy::Rect<LengthPercentage> {
    taffy::Rect {
        top:    LengthPercentage::Length(r.top),
        right:  LengthPercentage::Length(r.right),
        bottom: LengthPercentage::Length(r.bottom),
        left:   LengthPercentage::Length(r.left),
    }
}

fn to_taffy_rect_lpa(r: Rect) -> taffy::Rect<LengthPercentageAuto> {
    taffy::Rect {
        top:    LengthPercentageAuto::Length(r.top),
        right:  LengthPercentageAuto::Length(r.right),
        bottom: LengthPercentageAuto::Length(r.bottom),
        left:   LengthPercentageAuto::Length(r.left),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use fe_ui_core::style::{Dimension, Style};

    #[test]
    fn dimension_auto_converts() {
        assert!(matches!(to_taffy_dimension(Dimension::Auto), taffy::Dimension::Auto));
    }

    #[test]
    fn dimension_px_converts() {
        let r = to_taffy_dimension(Dimension::Px(200.0));
        assert!(matches!(r, taffy::Dimension::Length(v) if (v - 200.0).abs() < 1e-6));
    }

    #[test]
    fn dimension_percent_divides_by_100() {
        let r = to_taffy_dimension(Dimension::Percent(50.0));
        assert!(matches!(r, taffy::Dimension::Percent(v) if (v - 0.5).abs() < 1e-6));
    }

    #[test]
    fn dimension_percent_100_becomes_1() {
        let r = to_taffy_dimension(Dimension::Percent(100.0));
        assert!(matches!(r, taffy::Dimension::Percent(v) if (v - 1.0).abs() < 1e-6));
    }

    #[test]
    fn display_flex_converts() {
        assert!(matches!(to_taffy_display(Display::Flex), taffy::Display::Flex));
    }

    #[test]
    fn display_none_converts() {
        assert!(matches!(to_taffy_display(Display::None), taffy::Display::None));
    }

    #[test]
    fn flex_direction_column_converts() {
        assert!(matches!(
            to_taffy_flex_direction(FlexDirection::Column),
            taffy::FlexDirection::Column
        ));
    }

    #[test]
    fn align_items_center_converts() {
        assert!(matches!(
            to_taffy_align_items(AlignItems::Center),
            taffy::AlignItems::Center
        ));
    }

    #[test]
    fn justify_content_space_between_converts() {
        assert!(matches!(
            to_taffy_justify_content(JustifyContent::SpaceBetween),
            taffy::JustifyContent::SpaceBetween
        ));
    }

    #[test]
    fn to_taffy_style_size_maps_width_height() {
        let mut style = Style::default();
        style.width   = Dimension::Px(300.0);
        style.height  = Dimension::Px(150.0);
        let ts = to_taffy_style(&style);
        assert!(matches!(ts.size.width,  taffy::Dimension::Length(v) if (v - 300.0).abs() < 1e-6));
        assert!(matches!(ts.size.height, taffy::Dimension::Length(v) if (v - 150.0).abs() < 1e-6));
    }

    #[test]
    fn to_taffy_style_gap_applies_to_both_axes() {
        let mut style = Style::default();
        style.gap     = 16.0;
        let ts = to_taffy_style(&style);
        assert!(matches!(ts.gap.width,  LengthPercentage::Length(v) if (v - 16.0).abs() < 1e-6));
        assert!(matches!(ts.gap.height, LengthPercentage::Length(v) if (v - 16.0).abs() < 1e-6));
    }

    #[test]
    fn to_taffy_style_padding_maps_all_sides() {
        use fe_ui_core::style::Rect;
        let mut style = Style::default();
        style.padding = Rect { top: 8.0, right: 16.0, bottom: 8.0, left: 16.0 };
        let ts = to_taffy_style(&style);
        assert!(matches!(ts.padding.top,    LengthPercentage::Length(v) if (v - 8.0).abs()  < 1e-6));
        assert!(matches!(ts.padding.right,  LengthPercentage::Length(v) if (v - 16.0).abs() < 1e-6));
        assert!(matches!(ts.padding.bottom, LengthPercentage::Length(v) if (v - 8.0).abs()  < 1e-6));
        assert!(matches!(ts.padding.left,   LengthPercentage::Length(v) if (v - 16.0).abs() < 1e-6));
    }

    #[test]
    fn to_taffy_style_default_display_is_flex() {
        let ts = to_taffy_style(&Style::default());
        assert!(matches!(ts.display, taffy::Display::Flex));
    }
}
