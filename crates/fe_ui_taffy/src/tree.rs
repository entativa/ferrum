// crates/fe_ui_taffy/src/tree.rs
//
// The Ferrum layout tree — backed by Taffy.
//
// ─── Responsibilities ────────────────────────────────────────────────────────
//
//  LayoutTree owns the Taffy node tree. It mirrors the GhostWorld entity tree.
//  At the Sync Point, it receives the dirty EntityId set, recomputes only
//  affected subtrees, and returns a diff of changed LayoutRects.
//
// ─── The compute contract ────────────────────────────────────────────────────
//
//  compute() is called ONCE per frame at the Sync Point.
//  It receives:
//    - dirty:       &[EntityId]          — entities whose style changed
//    - styles:      &HashMap<EntityId, Style> — current styles for all entities
//    - parent_size: Size<Option<f32>>    — the viewport/container size
//
//  It returns:
//    - HashMap<EntityId, LayoutRect>     — ONLY entities whose rect changed
//                                          from the previous frame (diff)
//
//  Taffy recomputes exactly once per Sync Point.
//  Only changed rects reach Rapier — everything else costs nothing.
//
// ─── Node ownership ──────────────────────────────────────────────────────────
//
//  LayoutTree owns the mapping EntityId ↔ taffy::NodeId.
//  When an entity is registered, we create the Taffy node.
//  When an entity is despawned, we remove the Taffy node.
//
//  The raw taffy::NodeId is also stored back into GhostEntity.taffy_node
//  (as u64) by the frame loop after registration, so the rest of the system
//  can reference it without importing taffy.
//
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use fe_ui_core::{
    entity::EntityId,
    style::Style,
    types::LayoutRect,
};
use taffy::{
    geometry::Size,
    node::Taffy,
    prelude::NodeId,
    style::AvailableSpace,
};
use crate::{cache::LayoutCache, convert::to_taffy_style};

/// The Ferrum layout tree.
///
/// Owns a `taffy::Taffy` instance and a bidirectional mapping between
/// `EntityId` and `taffy::NodeId`. Called once per frame at the Sync Point.
pub struct LayoutTree {
    taffy:        Taffy,
    entity_to_node: HashMap<EntityId, NodeId>,
    node_to_entity: HashMap<NodeId, EntityId>,
    cache:          LayoutCache,
    /// The root node that all top-level entities are children of.
    /// Represents the viewport.
    root:           Option<NodeId>,
}

impl LayoutTree {
    pub fn new() -> Self {
        Self {
            taffy:          Taffy::new(),
            entity_to_node: HashMap::new(),
            node_to_entity: HashMap::new(),
            cache:          LayoutCache::new(),
            root:           None,
        }
    }

    /// Initialise the viewport root node.
    ///
    /// Must be called once before registering any entities.
    /// `width` and `height` are the viewport dimensions in logical pixels.
    pub fn init_viewport(&mut self, width: f32, height: f32) {
        use taffy::style::{Style as TaffyStyle, Dimension as TaffyDimension};
        use taffy::geometry::Size as TaffySize;

        let root_style = TaffyStyle {
            size: TaffySize {
                width:  TaffyDimension::Length(width),
                height: TaffyDimension::Length(height),
            },
            ..TaffyStyle::DEFAULT
        };

        let root = self.taffy.new_leaf(root_style)
            .expect("failed to create viewport root node");
        self.root = Some(root);
    }

    // ── Registration ─────────────────────────────────────────────────────────

    /// Register a new entity and its style with the layout tree.
    ///
    /// Creates a Taffy node for the entity. If `parent_id` is provided,
    /// appends this node as a child of the parent's Taffy node.
    ///
    /// Returns the raw NodeId as u64 — the frame loop stores this back into
    /// `GhostEntity.taffy_node` so other crates can reference it without
    /// importing taffy.
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

        // Wire into Taffy hierarchy
        match parent_id.and_then(|pid| self.entity_to_node.get(&pid).copied()) {
            Some(parent_node) => {
                self.taffy
                    .add_child(parent_node, node)
                    .expect("Taffy::add_child failed");
            }
            None => {
                // No parent — attach to viewport root if available
                if let Some(root) = self.root {
                    self.taffy
                        .add_child(root, node)
                        .expect("Taffy::add_child to root failed");
                }
            }
        }

        // Return NodeId as u64 for GhostEntity storage
        node_to_u64(node)
    }

    /// Remove an entity from the layout tree.
    ///
    /// Removes the Taffy node and its children recursively.
    /// Called when an entity is despawned.
    pub fn unregister(&mut self, id: EntityId) {
        if let Some(node) = self.entity_to_node.remove(&id) {
            self.node_to_entity.remove(&node);
            self.cache.remove(id);

            // Remove from Taffy — this also removes from its parent's child list
            let _ = self.taffy.remove(node);
        }
    }

    /// Update the style for an existing entity.
    ///
    /// Called when a `use_layout_signal` fires and the entity's style
    /// has been updated in the style map. Marks the node dirty in Taffy.
    pub fn update_style(&mut self, id: EntityId, style: &Style) {
        if let Some(&node) = self.entity_to_node.get(&id) {
            let taffy_style = to_taffy_style(style);
            let _ = self.taffy.set_style(node, taffy_style);
        }
    }

    // ── Compute ───────────────────────────────────────────────────────────────

    /// Compute layout for dirty entities. Returns only changed rects.
    ///
    /// Called ONCE per frame at the Sync Point.
    ///
    ///   1. Apply updated styles for all dirty entities.
    ///   2. Run Taffy layout from the root (Taffy handles dirty propagation
    ///      internally — it only recomputes nodes that need it).
    ///   3. Walk all registered entities, extract resolved rects.
    ///   4. Diff against cache — return only changed rects.
    ///
    /// The returned HashMap is the exact set handed to fe_ui_rapier to
    /// update spring targets. Unchanged entities cost nothing in physics.
    pub fn compute(
        &mut self,
        dirty:     &[EntityId],
        styles:    &HashMap<EntityId, Style>,
        viewport:  Size<f32>,
    ) -> HashMap<EntityId, LayoutRect> {
        // Step 1 — apply updated styles for dirty entities
        for &id in dirty {
            if let Some(style) = styles.get(&id) {
                self.update_style(id, style);
            }
        }

        // Step 2 — run Taffy layout
        // Taffy tracks its own dirty flags internally. Calling compute_layout
        // on the root causes it to recompute only nodes that were marked dirty
        // by set_style() above — not the whole tree.
        let available = Size {
            width:  AvailableSpace::Definite(viewport.width),
            height: AvailableSpace::Definite(viewport.height),
        };

        if let Some(root) = self.root {
            let _ = self.taffy.compute_layout(root, available);
        } else {
            // No root yet — compute each top-level node independently.
            // This path is used in tests without a viewport.
            let nodes: Vec<NodeId> = self.entity_to_node.values().copied().collect();
            for node in nodes {
                let _ = self.taffy.compute_layout(node, available);
            }
        }

        // Step 3 — extract resolved rects for all registered entities
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

    /// Get the last cached LayoutRect for an entity.
    /// Returns None if the entity hasn't been computed yet.
    pub fn get_rect(&self, id: EntityId) -> Option<&LayoutRect> {
        self.cache.get(id)
    }

    /// Returns the taffy NodeId for an entity as u64. Used for debugging.
    pub fn get_node(&self, id: EntityId) -> Option<u64> {
        self.entity_to_node.get(&id).copied().map(node_to_u64)
    }

    /// Number of registered entities.
    pub fn entity_count(&self) -> usize {
        self.entity_to_node.len()
    }

    /// Returns true if the entity is registered in the layout tree.
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

// ─── NodeId ↔ u64 ─────────────────────────────────────────────────────────────

/// Convert a taffy NodeId to a u64 for storage in GhostEntity.taffy_node.
/// NodeId is a newtype around a u64 internally — we extract it via index().
fn node_to_u64(node: NodeId) -> u64 {
    node.index() as u64
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;
    use fe_ui_core::{
        entity::EntityId,
        style::{Dimension, Style},
        types::LayoutRect,
    };
    use taffy::geometry::Size;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_id() -> (EntityId, SlotMap<EntityId, ()>) {
        let mut store = SlotMap::with_key();
        let id = store.insert(());
        (id, store)
    }

    fn px_style(width: f32, height: f32) -> Style {
        let mut s  = Style::default();
        s.width    = Dimension::Px(width);
        s.height   = Dimension::Px(height);
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
        let handle       = tree.register(id, &px_style(100.0, 50.0), None);
        // handle is non-zero — Taffy assigns from 1
        assert!(handle < u64::MAX);
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
        assert!(tree.contains(id));
        tree.unregister(id);
        assert!(!tree.contains(id));
        assert_eq!(tree.entity_count(), 0);
    }

    #[test]
    fn unregister_nonexistent_is_safe() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        // Should not panic
        tree.unregister(id);
    }

    #[test]
    fn register_with_parent_links_nodes() {
        let mut tree         = LayoutTree::new();
        let (parent, _sp)    = make_id();
        let (child, _sc)     = make_id();

        tree.register(parent, &px_style(300.0, 200.0), None);
        tree.register(child,  &px_style(100.0,  50.0), Some(parent));

        // Both registered
        assert!(tree.contains(parent));
        assert!(tree.contains(child));
    }

    // ── Compute ───────────────────────────────────────────────────────────────

    #[test]
    fn compute_resolves_fixed_size() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();

        tree.register(id, &px_style(200.0, 100.0), None);

        let styles    = HashMap::from([(id, px_style(200.0, 100.0))]);
        let changed   = tree.compute(&[id], &styles, viewport());

        // New entity — always in changed set
        assert!(changed.contains_key(&id));
        let rect = changed[&id];
        assert!((rect.width  - 200.0).abs() < 1.0,
            "expected width ~200, got {}", rect.width);
        assert!((rect.height - 100.0).abs() < 1.0,
            "expected height ~100, got {}", rect.height);
    }

    #[test]
    fn compute_second_frame_unchanged_not_in_diff() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();

        tree.register(id, &px_style(200.0, 100.0), None);
        let styles = HashMap::from([(id, px_style(200.0, 100.0))]);

        // First frame — new entity, in changed set
        tree.compute(&[id], &styles, viewport());

        // Second frame — nothing changed
        let changed = tree.compute(&[], &styles, viewport());
        assert!(!changed.contains_key(&id),
            "unchanged entity should not appear in second-frame diff");
    }

    #[test]
    fn compute_style_update_appears_in_diff() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();

        tree.register(id, &px_style(200.0, 100.0), None);
        let styles_v1 = HashMap::from([(id, px_style(200.0, 100.0))]);

        // First frame
        tree.compute(&[id], &styles_v1, viewport());

        // Update style — width changes to 300
        let styles_v2 = HashMap::from([(id, px_style(300.0, 100.0))]);
        let changed   = tree.compute(&[id], &styles_v2, viewport());

        assert!(changed.contains_key(&id),
            "style change should appear in diff");
        assert!((changed[&id].width - 300.0).abs() < 1.0,
            "expected width ~300, got {}", changed[&id].width);
    }

    #[test]
    fn compute_empty_dirty_set_still_returns_new_entities() {
        // Even with empty dirty set, new entities (not in cache) appear in diff
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();

        tree.register(id, &px_style(100.0, 50.0), None);
        let styles  = HashMap::from([(id, px_style(100.0, 50.0))]);

        // Dirty set is empty but entity is new
        let changed = tree.compute(&[], &styles, viewport());
        assert!(changed.contains_key(&id),
            "new entity should appear in diff even with empty dirty set");
    }

    #[test]
    fn get_rect_returns_last_computed() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();

        tree.register(id, &px_style(150.0, 75.0), None);
        let styles = HashMap::from([(id, px_style(150.0, 75.0))]);
        tree.compute(&[id], &styles, viewport());

        let rect = tree.get_rect(id).expect("rect should be cached after compute");
        assert!((rect.width  - 150.0).abs() < 1.0);
        assert!((rect.height -  75.0).abs() < 1.0);
    }

    #[test]
    fn get_rect_before_compute_returns_none() {
        let (id, _store) = make_id();
        let mut tree     = LayoutTree::new();
        tree.register(id, &px_style(100.0, 50.0), None);
        assert!(tree.get_rect(id).is_none());
    }

    #[test]
    fn compute_multiple_entities_all_resolved() {
        let mut tree      = LayoutTree::new();
        let (a, _store_a) = make_id();
        let (b, _store_b) = make_id();

        tree.register(a, &px_style(100.0, 50.0), None);
        tree.register(b, &px_style(200.0, 80.0), None);

        let styles = HashMap::from([
            (a, px_style(100.0, 50.0)),
            (b, px_style(200.0, 80.0)),
        ]);

        let changed = tree.compute(&[a, b], &styles, viewport());

        assert!(changed.contains_key(&a));
        assert!(changed.contains_key(&b));
    }

    #[test]
    fn compute_only_changed_entity_in_diff_on_update() {
        let mut tree      = LayoutTree::new();
        let (a, _store_a) = make_id();
        let (b, _store_b) = make_id();

        tree.register(a, &px_style(100.0, 50.0), None);
        tree.register(b, &px_style(200.0, 80.0), None);

        let styles_v1 = HashMap::from([
            (a, px_style(100.0, 50.0)),
            (b, px_style(200.0, 80.0)),
        ]);

        // First frame — both new, both in diff
        tree.compute(&[a, b], &styles_v1, viewport());

        // Second frame — only b's style changes
        let styles_v2 = HashMap::from([
            (a, px_style(100.0,  50.0)),  // unchanged
            (b, px_style(250.0,  80.0)),  // width changed
        ]);

        let changed = tree.compute(&[b], &styles_v2, viewport());

        assert!(!changed.contains_key(&a), "a unchanged — not in diff");
        assert!(changed.contains_key(&b),  "b changed — in diff");
    }
}
