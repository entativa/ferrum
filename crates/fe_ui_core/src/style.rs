//! style.rs — Layout style types (Flexbox / Grid / Block).
//! Mirrors Taffy's style model but owned by fe_ui_core so no
//! subsystem crate needs to import taffy directly for types.

use crate::types::Color;

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub display:         Display,
    pub flex_direction:  FlexDirection,
    pub flex_grow:       f32,
    pub flex_shrink:     f32,
    pub align_items:     AlignItems,
    pub justify_content: JustifyContent,
    pub width:           Dimension,
    pub height:          Dimension,
    pub min_width:       Dimension,
    pub min_height:      Dimension,
    pub max_width:       Dimension,
    pub max_height:      Dimension,
    pub padding:         Rect,
    pub margin:          Rect,
    pub gap:             f32,
    pub background:      Color,
    pub border_radius:   f32,
    pub border_width:    f32,
    pub border_color:    Color,
}

#[derive(Debug, Clone, Copy, Default)] pub enum Display    { #[default] Flex, Grid, Block, None }
#[derive(Debug, Clone, Copy, Default)] pub enum FlexDirection { #[default] Row, Column, RowReverse, ColumnReverse }
#[derive(Debug, Clone, Copy, Default)] pub enum AlignItems  { #[default] Stretch, Center, FlexStart, FlexEnd, Baseline }
#[derive(Debug, Clone, Copy, Default)] pub enum JustifyContent { #[default] FlexStart, Center, FlexEnd, SpaceBetween, SpaceAround, SpaceEvenly }

#[derive(Debug, Clone, Copy, Default)]
pub enum Dimension {
    #[default] Auto,
    Px(f32),
    Percent(f32),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub top:    f32,
    pub right:  f32,
    pub bottom: f32,
    pub left:   f32,
}

impl Rect {
    pub fn all(v: f32) -> Self { Self { top: v, right: v, bottom: v, left: v } }
}
