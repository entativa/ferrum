// crates/fe_ui_taffy/src/cache.rs
//
// Layout result cache for fe_ui_taffy.
//
// ─── What this does ──────────────────────────────────────────────────────────
//
//  After Taffy resolves layout, we store the resulting LayoutRect per entity.
//  On the next frame, we diff the new results against the cache to find which
//  entities actually changed position or size.
//
//  Only changed entities update their Rapier spring targets.
//  An entity whose layout rect is identical to last frame costs nothing
//  in the physics layer — no spring target update, no unnecessary force.
//
// ─── The diff ────────────────────────────────────────────────────────────────
//
//  Two rects are considered equal if all four fields are within EPSILON of
//  each other. We use a small epsilon rather than exact float comparison
//  because Taffy's internal arithmetic can produce sub-pixel drift between
//  frames even when nothing changed logically.
//
//  RECT_CHANGE_EPSILON = 0.01px — below this threshold, we consider the
//  layout unchanged. This is well below the SLEEP_THRESHOLD_PX in solver.rs
//  (0.1px) so we never miss a layout change that would affect physics.
//
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use fe_ui_core::{entity::EntityId, types::LayoutRect};

/// Below this pixel delta, two rects are considered equal.
/// Sub-pixel drift from Taffy's arithmetic is ignored.
const RECT_CHANGE_EPSILON: f32 = 0.01;

/// Cached layout results from the previous Sync Point.
///
/// Owned by `LayoutTree`. Updated every frame after Taffy computes.
#[derive(Debug, Default)]
pub struct LayoutCache {
    rects: HashMap<EntityId, LayoutRect>,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            rects: HashMap::new(),
        }
    }

    /// Insert or update the cached rect for an entity.
    pub fn set(&mut self, id: EntityId, rect: LayoutRect) {
        self.rects.insert(id, rect);
    }

    /// Get the cached rect for an entity. Returns None if not yet cached.
    pub fn get(&self, id: EntityId) -> Option<&LayoutRect> {
        self.rects.get(&id)
    }

    /// Remove an entity from the cache. Called on despawn.
    pub fn remove(&mut self, id: EntityId) {
        self.rects.remove(&id);
    }

    /// Returns true if the entity has a cached rect.
    pub fn contains(&self, id: EntityId) -> bool {
        self.rects.contains_key(&id)
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.rects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    /// Diff a new set of layout results against the cache.
    ///
    /// Returns only the entities whose rect changed beyond RECT_CHANGE_EPSILON
    /// since the last frame, alongside their new rect.
    ///
    /// This is the set handed to fe_ui_rapier to update spring targets.
    /// Entities with unchanged rects are not included — their springs keep
    /// their existing target with no update cost.
    ///
    /// Also updates the cache with the new values.
    pub fn diff_and_update(
        &mut self,
        new_rects: &HashMap<EntityId, LayoutRect>,
    ) -> HashMap<EntityId, LayoutRect> {
        let mut changed = HashMap::new();

        for (&id, &new_rect) in new_rects {
            let is_changed = match self.rects.get(&id) {
                // New entity — always include
                None => true,
                // Existing entity — include only if rect changed
                Some(&cached) => rect_changed(&cached, &new_rect),
            };

            if is_changed {
                changed.insert(id, new_rect);
            }

            // Always update the cache to the latest value
            self.rects.insert(id, new_rect);
        }

        changed
    }

    /// Clear the entire cache. Called on full scene reload.
    pub fn clear(&mut self) {
        self.rects.clear();
    }
}

/// Returns true if two rects differ by more than RECT_CHANGE_EPSILON
/// on any field.
fn rect_changed(a: &LayoutRect, b: &LayoutRect) -> bool {
    (a.x      - b.x).abs()      > RECT_CHANGE_EPSILON
        || (a.y      - b.y).abs()      > RECT_CHANGE_EPSILON
        || (a.width  - b.width).abs()  > RECT_CHANGE_EPSILON
        || (a.height - b.height).abs() > RECT_CHANGE_EPSILON
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;
    use fe_ui_core::entity::EntityId;

    fn make_id() -> (EntityId, SlotMap<EntityId, ()>) {
        let mut store = SlotMap::with_key();
        let id = store.insert(());
        (id, store)
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> LayoutRect {
        LayoutRect::new(x, y, w, h)
    }

    // ── Basic get/set ─────────────────────────────────────────────────────────

    #[test]
    fn set_and_get() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));
        let r = cache.get(id).unwrap();
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 50.0);
    }

    #[test]
    fn get_missing_returns_none() {
        let (id, _store) = make_id();
        let cache        = LayoutCache::new();
        assert!(cache.get(id).is_none());
    }

    #[test]
    fn set_updates_existing() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));
        cache.set(id, rect(0.0, 0.0, 200.0, 50.0));
        assert_eq!(cache.get(id).unwrap().width, 200.0);
    }

    #[test]
    fn remove_clears_entry() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));
        cache.remove(id);
        assert!(cache.get(id).is_none());
    }

    #[test]
    fn contains_returns_correct_value() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        assert!(!cache.contains(id));
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));
        assert!(cache.contains(id));
    }

    #[test]
    fn len_tracks_entries() {
        let mut cache      = LayoutCache::new();
        let (a, _store_a)  = make_id();
        let (b, _store_b)  = make_id();

        assert_eq!(cache.len(), 0);
        cache.set(a, rect(0.0, 0.0, 100.0, 50.0));
        assert_eq!(cache.len(), 1);
        cache.set(b, rect(0.0, 0.0, 200.0, 50.0));
        assert_eq!(cache.len(), 2);
        cache.remove(a);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn clear_empties_cache() {
        let mut cache     = LayoutCache::new();
        let (id, _store)  = make_id();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));
        cache.clear();
        assert!(cache.is_empty());
    }

    // ── diff_and_update ───────────────────────────────────────────────────────

    #[test]
    fn diff_new_entity_always_included() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();

        let new_rects = HashMap::from([(id, rect(0.0, 0.0, 100.0, 50.0))]);
        let changed   = cache.diff_and_update(&new_rects);

        assert!(changed.contains_key(&id));
    }

    #[test]
    fn diff_unchanged_entity_not_included() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();

        let r = rect(0.0, 0.0, 100.0, 50.0);
        cache.set(id, r);

        // Same rect — should NOT appear in changed set
        let new_rects = HashMap::from([(id, r)]);
        let changed   = cache.diff_and_update(&new_rects);

        assert!(!changed.contains_key(&id));
    }

    #[test]
    fn diff_changed_x_included() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));

        let new_rects = HashMap::from([(id, rect(50.0, 0.0, 100.0, 50.0))]);
        let changed   = cache.diff_and_update(&new_rects);

        assert!(changed.contains_key(&id));
        assert_eq!(changed[&id].x, 50.0);
    }

    #[test]
    fn diff_changed_width_included() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));

        let new_rects = HashMap::from([(id, rect(0.0, 0.0, 200.0, 50.0))]);
        let changed   = cache.diff_and_update(&new_rects);

        assert!(changed.contains_key(&id));
    }

    #[test]
    fn diff_sub_epsilon_change_not_included() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));

        // Change smaller than RECT_CHANGE_EPSILON (0.01)
        let new_rects = HashMap::from([(id, rect(0.005, 0.0, 100.0, 50.0))]);
        let changed   = cache.diff_and_update(&new_rects);

        assert!(!changed.contains_key(&id));
    }

    #[test]
    fn diff_exactly_epsilon_not_included() {
        // Exactly at epsilon is NOT considered changed (strict >)
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));

        let new_rects = HashMap::from([(id, rect(0.01, 0.0, 100.0, 50.0))]);
        let changed   = cache.diff_and_update(&new_rects);

        assert!(!changed.contains_key(&id));
    }

    #[test]
    fn diff_just_above_epsilon_included() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));

        let new_rects = HashMap::from([(id, rect(0.011, 0.0, 100.0, 50.0))]);
        let changed   = cache.diff_and_update(&new_rects);

        assert!(changed.contains_key(&id));
    }

    #[test]
    fn diff_updates_cache_regardless_of_change() {
        let (id, _store) = make_id();
        let mut cache    = LayoutCache::new();
        cache.set(id, rect(0.0, 0.0, 100.0, 50.0));

        // Sub-epsilon change — NOT in changed set
        let new_rect  = rect(0.005, 0.0, 100.0, 50.0);
        let new_rects = HashMap::from([(id, new_rect)]);
        cache.diff_and_update(&new_rects);

        // But cache IS updated to the new value
        let cached = cache.get(id).unwrap();
        assert!((cached.x - 0.005).abs() < 1e-6);
    }

    #[test]
    fn diff_multiple_entities_only_changed_returned() {
        let mut cache       = LayoutCache::new();
        let (a, _store_a)   = make_id();
        let (b, _store_b)   = make_id();
        let (c, _store_c)   = make_id();

        cache.set(a, rect(0.0,  0.0, 100.0, 50.0));
        cache.set(b, rect(0.0, 50.0, 100.0, 50.0));
        // c is new — not in cache yet

        let new_rects = HashMap::from([
            (a, rect(0.0, 0.0, 100.0, 50.0)),   // unchanged
            (b, rect(0.0, 60.0, 100.0, 50.0)),  // moved y: 50 → 60
            (c, rect(0.0, 0.0,  50.0, 25.0)),   // new
        ]);

        let changed = cache.diff_and_update(&new_rects);

        assert!(!changed.contains_key(&a), "a unchanged — should not be in diff");
        assert!(changed.contains_key(&b),  "b moved — should be in diff");
        assert!(changed.contains_key(&c),  "c is new — should be in diff");
        assert_eq!(changed.len(), 2);
    }

    #[test]
    fn diff_empty_new_rects_returns_empty() {
        let mut cache = LayoutCache::new();
        let changed   = cache.diff_and_update(&HashMap::new());
        assert!(changed.is_empty());
    }

    // ── rect_changed helper ───────────────────────────────────────────────────

    #[test]
    fn rect_changed_identical_is_false() {
        let r = rect(10.0, 20.0, 100.0, 50.0);
        assert!(!rect_changed(&r, &r));
    }

    #[test]
    fn rect_changed_different_x_is_true() {
        let a = rect(0.0,  0.0, 100.0, 50.0);
        let b = rect(1.0,  0.0, 100.0, 50.0);
        assert!(rect_changed(&a, &b));
    }

    #[test]
    fn rect_changed_different_y_is_true() {
        let a = rect(0.0, 0.0, 100.0, 50.0);
        let b = rect(0.0, 1.0, 100.0, 50.0);
        assert!(rect_changed(&a, &b));
    }

    #[test]
    fn rect_changed_different_height_is_true() {
        let a = rect(0.0, 0.0, 100.0,  50.0);
        let b = rect(0.0, 0.0, 100.0, 100.0);
        assert!(rect_changed(&a, &b));
    }
}
