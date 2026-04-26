// crates/fe_ui_taffy/src/tree.rs
//
// The Ferrum layout tree — backed by Taffy 0.4.4.
//
// ─── API notes for taffy 0.4.4 ───────────────────────────────────────────────
//
//  taffy 0.4.x renamed and reorganised significantly from 0.3.x:
//    TaffyTree<NodeContext>  replaces  Taffy
//    tree.new_leaf(style)    unchanged
//    tree.add_child(p, c)    unchanged
//    tree.remove(node)       unchanged
//    tree.set_style(n, s)    unchanged
//    tree.compute_layout(root, available_space)  unchanged
//    tree.layout(node)       returns &Layout
//    NodeId                  is now a slotmap key — use as opaque handle
//    NodeId as u64           cast via u64::from(node) in 0.4.x
//    AvailableSpace          lives at taffy::AvailableSpace
//
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use fe_ui_core::{
    entity::EntityId,
    style::Style,
    types::LayoutRect,
};
use taffy::prelude::*;
use crate::{cache::LayoutCache, convert::to_taffy_style};

/// The Ferrum layout tree.
///
/// Owns a `TaffyTree` and a bidirectional mapping between
/// `EntityId` and `taffy::NodeId`. Called once per frame at the Sync Point.
pub struct LayoutTree {
    taffy:           TaffyTree<()>,
    entity_to_node:  HashMap<EntityId, NodeId>,
    node_to_entity:  HashMap<NodeId, EntityId>,
    cache:           LayoutCache,
    root:            Option<NodeId>,
}

impl LayoutTree {
    pub fn new() -> Self {
        Self {
            taffy:          TaffyTree::new(),
            entity_to_node: HashMap::new(),
            node_to_entity: HashMap::new(),
            cache:          LayoutCache::new(),
            root:           None,
        }
    }

    /// Initialise the viewport root node.
    /// Must be called once before registering entities.
    pub fn init_viewport(&mut self, width: f32, height: f32) {
        let root_style = taffy::Style {
            size: Size {
                width:  Dimension::Length(width),
                height: Dimension::Length(height),
            },
            ..taffy::Style::DEFAULT
        };
        let root = self.taffy
            .new_leaf(root_style)
            .expect("failed to create viewport root node");
        self.root = Some(root);
    }

    // ── Registration ─────────────────────────────────────────────────────────

    /// Register a new entity. Creates a Taffy node.
    /// Returns the NodeId as u64 for storage in GhostEntity.taffy_node.
    pub fn register(
        &mut self,
        id:        EntityId,
        style:     &Style,
        parent_id: Option<EntityId>,
    ) -> u64 {
        let taffy_style = to_taffy_style(style);
        let node = self.taffy
            .new_leaf(taffy_style)
            .expect("Taffy::new_leaf failed");

        self.entity_to_node.insert(id, node);
        self.node_to_entity.insert(node, id);

        match parent_id.and_then(|pid| self.entity_to_node.get(&pid).copied()) {
            Some(parent_node) => {
                self.taffy
                    .add_child(parent_node, node)
                    .expect("Taffy::add_child failed");
            }
            None => {
                if let Some(root) = self.root {
                    self.taffy
                        .add_child(root, node)
                        .expect("Taffy::add_child to root failed");
                }
            }
        }

        node_to_u64(node)
    }

    /// Remove an entity from the layout tree.
    pub fn unregister(&mut self, id: EntityId) {
        if let Some(node) = self.entity_to_node.remove(&id) {
            self.node_to_entity.remove(&node);
            self.cache.remove(id);
            let _ = self.taffy.remove(node);
        }
    }

    /// Update the style for an existing entity.
    pub fn update_style(&mut self, id: EntityId, style: &Style) {
        if let Some(&node) = self.entity_to_node.get(&id) {
            let taffy_style = to_taffy_style(style);
            let _ = self.taffy.set_style(node, taffy_style);
        }
    }

    // ── Compute ───────────────────────────────────────────────────────────────

    /// Compute layout for dirty entities. Returns only changed rects.
    /// Called ONCE per frame at the Sync Point.
    pub fn compute(
        &mut self,
        dirty:    &[EntityId],
        styles:   &HashMap<EntityId, Style>,
        viewport: Size<f32>,
    ) -> HashMap<EntityId, LayoutRect> {
        // Step 1 — apply updated styles for dirty entities
        for &id in dirty {
            if let Some(style) = styles.get(&id) {
                self.update_style(id, style);
            }
        }

        // Step 2 — run Taffy layout
        let available = Size {
            width:  AvailableSpace::Definite(viewport.width),
            height: AvailableSpace::Definite(viewport.height),
        };

        if let Some(root) = self.root {
            let _ = self.taffy.compute_layout(root, available);
        } else {
            // No root — compute each top-level node independently (used in tests)
            let nodes: Vec<NodeId> = self.entity_to_node.values().copied().collect();
            for node in nodes {
                let _ = self.taffy.compute_layout(node, available);
            }
        }

        // Step 3 — extract resolved rects
        let mut new_rects = HashMap::new();
        for (&id, &node) in &self.entity_to_node {
            if let Ok(layout) = self.taffy.layout(node) {
                let rect = LayoutRect::new(
                    layout.location.x,
                    layout.location.y,
                    layout.size.width,
                    layout.size.height,
                );
                new_rects.insert(id, rect);
            }
        }

        // Step 4 — diff against cache, return only changed rects
        self.cache.diff_and_update(&new_rects)
    }

    // ── Reads ─────────────────────────────────────────────────────────────────

    pub fn get_rect(&self, id: EntityId) -> Option<&LayoutRect> {
        self.cache.get(id)
    }

    pub fn get_node(&self, id: EntityId) -> Option<u64> {
        self.entity_to_node.get(&id).copied().map(node_to_u64)
    }

    pub fn entity_count(&self) -> usize {
        self.entity_to_node.len()
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.entity_to_node.contains_key(&id)
    }
}

impl Default for LayoutTree {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Debug for LayoutTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutTree")
            .field("entities", &self.entity_to_node.len())
            .field("cache",    &self.cache.len())
            .finish()
    }
}

// ─── NodeId → u64 ─────────────────────────────────────────────────────────────

/// Convert a taffy NodeId to u64 for storage in GhostEntity.taffy_node.
/// In taffy 0.4.x NodeId is a slotmap key — u64::from() is the correct cast.
fn node_to_u64(node: NodeId) -> u64 {
    u64::from(node)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;
    use fe_ui_core::{
        entity::EntityId,
        style::{Dimension, Style},
    };

    fn make_id() -> (EntityId, SlotMap<EntityId, ()>) {
        let mut store = SlotMap::with_key();
        let id = store.insert(());
        (id, store)
    }

    fn px_style(width: f32, height: f32) -> Style {
        let mut s = Style::default();
        s.width   = Dimension::Px(width);
        s.height  = Dimension::Px(height);
        s
    }

    fn viewport() -> Size<f32> {
        Size { width: 1280.0, height: 800.0 }
    }

    // ── Registration ─────────────────────────────────────────────────────────

    #[test]
    fn register_returns_node_handle() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        let _handle      = tree.register(id, &px_style(100.0, 50.0), None);
        assert!(tree.contains(id));
    }

    #[test]
    fn register_increments_entity_count() {
        let mut tree      = LayoutTree::new();
        let (a, _store_a) = make_id();
        let (b, _store_b) = make_id();
        assert_eq!(tree.entity_count(), 0);
        tree.register(a, &px_style(100.0, 50.0), None);
        assert_eq!(tree.entity_count(), 1);
        tree.register(b, &px_style(200.0, 50.0), None);
        assert_eq!(tree.entity_count(), 2);
    }

    #[test]
    fn unregister_removes_entity() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        tree.register(id, &px_style(100.0, 50.0), None);
        tree.unregister(id);
        assert!(!tree.contains(id));
        assert_eq!(tree.entity_count(), 0);
    }

    #[test]
    fn unregister_nonexistent_is_safe() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        tree.unregister(id); // should not panic
    }

    #[test]
    fn register_with_parent_links_nodes() {
        let mut tree      = LayoutTree::new();
        let (parent, _sp) = make_id();
        let (child, _sc)  = make_id();
        tree.register(parent, &px_style(300.0, 200.0), None);
        tree.register(child,  &px_style(100.0,  50.0), Some(parent));
        assert!(tree.contains(parent));
        assert!(tree.contains(child));
    }

    // ── Compute ───────────────────────────────────────────────────────────────

    #[test]
    fn compute_resolves_fixed_size() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        tree.register(id, &px_style(200.0, 100.0), None);

        let styles  = HashMap::from([(id, px_style(200.0, 100.0))]);
        let changed = tree.compute(&[id], &styles, viewport());

        assert!(changed.contains_key(&id));
        let rect = changed[&id];
        assert!((rect.width  - 200.0).abs() < 1.0, "width: {}", rect.width);
        assert!((rect.height - 100.0).abs() < 1.0, "height: {}", rect.height);
    }

    #[test]
    fn compute_second_frame_unchanged_not_in_diff() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        tree.register(id, &px_style(200.0, 100.0), None);
        let styles = HashMap::from([(id, px_style(200.0, 100.0))]);

        // First frame — new, in diff
        tree.compute(&[id], &styles, viewport());
        // Second frame — unchanged
        let changed = tree.compute(&[], &styles, viewport());
        assert!(!changed.contains_key(&id));
    }

    #[test]
    fn compute_style_update_appears_in_diff() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        tree.register(id, &px_style(200.0, 100.0), None);

        let styles_v1 = HashMap::from([(id, px_style(200.0, 100.0))]);
        tree.compute(&[id], &styles_v1, viewport());

        let styles_v2 = HashMap::from([(id, px_style(300.0, 100.0))]);
        let changed   = tree.compute(&[id], &styles_v2, viewport());

        assert!(changed.contains_key(&id));
        assert!((changed[&id].width - 300.0).abs() < 1.0);
    }

    #[test]
    fn compute_new_entity_in_diff_even_with_empty_dirty() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        tree.register(id, &px_style(100.0, 50.0), None);
        let styles  = HashMap::from([(id, px_style(100.0, 50.0))]);
        let changed = tree.compute(&[], &styles, viewport());
        assert!(changed.contains_key(&id));
    }

    #[test]
    fn get_rect_returns_last_computed() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        tree.register(id, &px_style(150.0, 75.0), None);
        let styles = HashMap::from([(id, px_style(150.0, 75.0))]);
        tree.compute(&[id], &styles, viewport());
        let rect = tree.get_rect(id).unwrap();
        assert!((rect.width  - 150.0).abs() < 1.0);
        assert!((rect.height -  75.0).abs() < 1.0);
    }

    #[test]
    fn get_rect_before_compute_is_none() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        tree.register(id, &px_style(100.0, 50.0), None);
        assert!(tree.get_rect(id).is_none());
    }

    #[test]
    fn compute_only_changed_entity_in_diff() {
        let mut tree      = LayoutTree::new();
        let (a, _store_a) = make_id();
        let (b, _store_b) = make_id();

        tree.register(a, &px_style(100.0, 50.0), None);
        tree.register(b, &px_style(200.0, 80.0), None);

        let styles_v1 = HashMap::from([
            (a, px_style(100.0, 50.0)),
            (b, px_style(200.0, 80.0)),
        ]);
        tree.compute(&[a, b], &styles_v1, viewport());

        // Only b changes
        let styles_v2 = HashMap::from([
            (a, px_style(100.0, 50.0)),  // unchanged
            (b, px_style(250.0, 80.0)),  // changed
        ]);
        let changed = tree.compute(&[b], &styles_v2, viewport());

        assert!(!changed.contains_key(&a), "a unchanged");
        assert!(changed.contains_key(&b),  "b changed");
    }
}
