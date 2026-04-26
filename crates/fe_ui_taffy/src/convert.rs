// crates/fe_ui_taffy/src/convert.rs
//
// fe_ui_core::Style → taffy::Style conversion.
//
// ─── Why this exists ─────────────────────────────────────────────────────────
//
//  fe_ui_core defines its own Style type so that no subsystem crate needs
//  to import another subsystem's types. fe_ui_taffy is the only crate that
//  imports both fe_ui_core and taffy — so the conversion lives here.
//
//  The mapping is mostly mechanical but a few decisions are load-bearing:
//
//  Dimension::Auto
//    Maps to taffy::Dimension::Auto. Taffy will size the widget based on
//    its content. This is the correct default for most widgets.
//
//  Dimension::Px(f32)
//    Maps to taffy::Dimension::Length(f32). Taffy uses "length" for
//    device-independent pixel values (same coordinate space as Ferrum).
//
//  Dimension::Percent(f32)
//    Maps to taffy::Dimension::Percent(f32 / 100.0). Taffy expects [0,1]
//    while Ferrum stores [0,100] to match CSS conventions.
//
//  Padding / Margin
//    fe_ui_core::Rect maps to taffy::Rect<LengthPercentage>.
//    We use LengthPercentage::Length for all values since fe_ui_core
//    doesn't yet distinguish padding-percent from padding-px.
//
//  Gap
//    fe_ui_core stores gap as a single f32. Taffy wants a Size<LengthPercentage>
//    (row gap, column gap). We apply the single value to both axes.
//
// ─────────────────────────────────────────────────────────────────────────────

use fe_ui_core::style::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, Rect, Style,
};
use taffy::{
    geometry::{Rect as TaffyRect, Size},
    style::{
        AlignItems as TaffyAlignItems,
        Dimension as TaffyDimension,
        Display as TaffyDisplay,
        FlexDirection as TaffyFlexDirection,
        JustifyContent as TaffyJustifyContent,
        LengthPercentage,
        LengthPercentageAuto,
        Style as TaffyStyle,
    },
};

/// Convert a `fe_ui_core::Style` to a `taffy::Style`.
///
/// Called by `LayoutTree::register()` and `LayoutTree::compute()` whenever
/// a dirty entity's style needs to be applied to the Taffy tree.
pub fn to_taffy_style(style: &Style) -> TaffyStyle {
    TaffyStyle {
        display:          to_taffy_display(style.display),
        flex_direction:   to_taffy_flex_direction(style.flex_direction),
        flex_grow:        style.flex_grow,
        flex_shrink:      style.flex_shrink,
        align_items:      Some(to_taffy_align_items(style.align_items)),
        justify_content:  Some(to_taffy_justify_content(style.justify_content)),
        size:             Size {
            width:  to_taffy_dimension(style.width),
            height: to_taffy_dimension(style.height),
        },
        min_size:         Size {
            width:  to_taffy_dimension(style.min_width),
            height: to_taffy_dimension(style.min_height),
        },
        max_size:         Size {
            width:  to_taffy_dimension(style.max_width),
            height: to_taffy_dimension(style.max_height),
        },
        padding:          to_taffy_rect_lp(style.padding),
        margin:           to_taffy_rect_lpa(style.margin),
        gap:              Size {
            width:  LengthPercentage::Length(style.gap),
            height: LengthPercentage::Length(style.gap),
        },
        ..TaffyStyle::DEFAULT
    }
}

// ─── Individual converters ────────────────────────────────────────────────────

fn to_taffy_display(d: Display) -> TaffyDisplay {
    match d {
        Display::Flex  => TaffyDisplay::Flex,
        Display::Grid  => TaffyDisplay::Grid,
        Display::Block => TaffyDisplay::Block,
        Display::None  => TaffyDisplay::None,
    }
}

fn to_taffy_flex_direction(d: FlexDirection) -> TaffyFlexDirection {
    match d {
        FlexDirection::Row           => TaffyFlexDirection::Row,
        FlexDirection::Column        => TaffyFlexDirection::Column,
        FlexDirection::RowReverse    => TaffyFlexDirection::RowReverse,
        FlexDirection::ColumnReverse => TaffyFlexDirection::ColumnReverse,
    }
}

fn to_taffy_align_items(a: AlignItems) -> TaffyAlignItems {
    match a {
        AlignItems::Stretch   => TaffyAlignItems::Stretch,
        AlignItems::Center    => TaffyAlignItems::Center,
        AlignItems::FlexStart => TaffyAlignItems::FlexStart,
        AlignItems::FlexEnd   => TaffyAlignItems::FlexEnd,
        AlignItems::Baseline  => TaffyAlignItems::Baseline,
    }
}

fn to_taffy_justify_content(j: JustifyContent) -> TaffyJustifyContent {
    match j {
        JustifyContent::FlexStart    => TaffyJustifyContent::FlexStart,
        JustifyContent::Center       => TaffyJustifyContent::Center,
        JustifyContent::FlexEnd      => TaffyJustifyContent::FlexEnd,
        JustifyContent::SpaceBetween => TaffyJustifyContent::SpaceBetween,
        JustifyContent::SpaceAround  => TaffyJustifyContent::SpaceAround,
        JustifyContent::SpaceEvenly  => TaffyJustifyContent::SpaceEvenly,
    }
}

/// Convert a `fe_ui_core::Dimension` to a `taffy::Dimension`.
///
/// Percent values are divided by 100 — fe_ui_core stores [0,100] (CSS
/// convention) while Taffy expects [0,1].
pub fn to_taffy_dimension(d: Dimension) -> TaffyDimension {
    match d {
        Dimension::Auto       => TaffyDimension::Auto,
        Dimension::Px(px)     => TaffyDimension::Length(px),
        Dimension::Percent(p) => TaffyDimension::Percent(p / 100.0),
    }
}

/// Convert a `fe_ui_core::Rect` to a `taffy::Rect<LengthPercentage>`.
/// Used for padding.
fn to_taffy_rect_lp(r: Rect) -> TaffyRect<LengthPercentage> {
    TaffyRect {
        top:    LengthPercentage::Length(r.top),
        right:  LengthPercentage::Length(r.right),
        bottom: LengthPercentage::Length(r.bottom),
        left:   LengthPercentage::Length(r.left),
    }
}

/// Convert a `fe_ui_core::Rect` to a `taffy::Rect<LengthPercentageAuto>`.
/// Used for margin (which can be Auto in CSS).
fn to_taffy_rect_lpa(r: Rect) -> TaffyRect<LengthPercentageAuto> {
    TaffyRect {
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
        assert!(matches!(
            to_taffy_dimension(Dimension::Auto),
            TaffyDimension::Auto
        ));
    }

    #[test]
    fn dimension_px_converts() {
        let result = to_taffy_dimension(Dimension::Px(200.0));
        assert!(matches!(result, TaffyDimension::Length(v) if (v - 200.0).abs() < 1e-6));
    }

    #[test]
    fn dimension_percent_divides_by_100() {
        // fe_ui_core stores 50.0 meaning "50%"
        // taffy expects 0.5
        let result = to_taffy_dimension(Dimension::Percent(50.0));
        assert!(matches!(result, TaffyDimension::Percent(v) if (v - 0.5).abs() < 1e-6));
    }

    #[test]
    fn dimension_percent_100_becomes_1() {
        let result = to_taffy_dimension(Dimension::Percent(100.0));
        assert!(matches!(result, TaffyDimension::Percent(v) if (v - 1.0).abs() < 1e-6));
    }

    #[test]
    fn display_flex_converts() {
        assert!(matches!(to_taffy_display(Display::Flex), TaffyDisplay::Flex));
    }

    #[test]
    fn display_none_converts() {
        assert!(matches!(to_taffy_display(Display::None), TaffyDisplay::None));
    }

    #[test]
    fn flex_direction_row_converts() {
        assert!(matches!(
            to_taffy_flex_direction(FlexDirection::Row),
            TaffyFlexDirection::Row
        ));
    }

    #[test]
    fn flex_direction_column_converts() {
        assert!(matches!(
            to_taffy_flex_direction(FlexDirection::Column),
            TaffyFlexDirection::Column
        ));
    }

    #[test]
    fn align_items_center_converts() {
        assert!(matches!(
            to_taffy_align_items(AlignItems::Center),
            TaffyAlignItems::Center
        ));
    }

    #[test]
    fn justify_content_space_between_converts() {
        assert!(matches!(
            to_taffy_justify_content(JustifyContent::SpaceBetween),
            TaffyJustifyContent::SpaceBetween
        ));
    }

    #[test]
    fn to_taffy_style_size_maps_width_height() {
        let mut style  = Style::default();
        style.width    = Dimension::Px(300.0);
        style.height   = Dimension::Px(150.0);
        let ts = to_taffy_style(&style);
        assert!(matches!(ts.size.width,  TaffyDimension::Length(v) if (v - 300.0).abs() < 1e-6));
        assert!(matches!(ts.size.height, TaffyDimension::Length(v) if (v - 150.0).abs() < 1e-6));
    }

    #[test]
    fn to_taffy_style_gap_applies_to_both_axes() {
        let mut style = Style::default();
        style.gap = 16.0;
        let ts = to_taffy_style(&style);
        assert!(matches!(ts.gap.width,  LengthPercentage::Length(v) if (v - 16.0).abs() < 1e-6));
        assert!(matches!(ts.gap.height, LengthPercentage::Length(v) if (v - 16.0).abs() < 1e-6));
    }

    #[test]
    fn to_taffy_style_padding_maps_all_sides() {
        use fe_ui_core::style::Rect;
        let mut style  = Style::default();
        style.padding  = Rect { top: 8.0, right: 16.0, bottom: 8.0, left: 16.0 };
        let ts = to_taffy_style(&style);
        assert!(matches!(ts.padding.top,    LengthPercentage::Length(v) if (v - 8.0).abs()  < 1e-6));
        assert!(matches!(ts.padding.right,  LengthPercentage::Length(v) if (v - 16.0).abs() < 1e-6));
        assert!(matches!(ts.padding.bottom, LengthPercentage::Length(v) if (v - 8.0).abs()  < 1e-6));
        assert!(matches!(ts.padding.left,   LengthPercentage::Length(v) if (v - 16.0).abs() < 1e-6));
    }

    #[test]
    fn to_taffy_style_default_display_is_flex() {
        let style = Style::default();
        let ts    = to_taffy_style(&style);
        assert!(matches!(ts.display, TaffyDisplay::Flex));
    }
}
