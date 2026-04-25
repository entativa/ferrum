//! tree.rs — The Ferrum layout tree, backed by Taffy.

use std::collections::HashMap;
use fe_ui_core::{EntityId, types::LayoutRect, style::Style};

pub struct LayoutTree {}

impl LayoutTree {
    pub fn new() -> Self { Self {} }

    pub fn register(&mut self, _id: EntityId, _style: &Style) {
        // TODO: create taffy node, store EntityId → NodeId mapping
    }

    pub fn unregister(&mut self, _id: EntityId) {
        // TODO: remove taffy node
    }

    pub fn compute(
        &mut self,
        _dirty:  &[EntityId],
        _styles: &HashMap<EntityId, Style>,
    ) -> HashMap<EntityId, LayoutRect> {
        HashMap::new()
    }
}
