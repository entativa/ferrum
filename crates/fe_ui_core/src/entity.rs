//! entity.rs — The Ghost ECS.
//!
//! GhostEntity maps a single widget across all three subsystems:
//!   - Taffy NodeId        (layout)
//!   - Rapier BodyHandle   (physics)
//!   - wgpu instance index (rendering)
//!
//! # Visibility
//! GhostWorld is pub(crate) only. The #[component] macro is the
//! only door in. If you need direct entity access, the public API
//! has a gap — open an issue rather than reaching in here.

use slotmap::{SlotMap, new_key_type};
use crate::{style::Style, physics::PhysicsProps, types::LayoutRect};

new_key_type! {
    /// The only shared identity across layout, physics, and rendering.
    /// Never exposed in a public API.
    pub struct EntityId;
}

/// The joint type between a child body and its parent body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JointType {
    /// Default — child is rigidly welded to parent. Moves identically.
    Fixed,
    /// Child follows parent via spring force. Lags, jiggles, settles.
    /// Activated when the child declares Physics::dynamic().
    Spring,
    /// Child is detached. Governed only by its own forces.
    Free,
}

impl Default for JointType {
    fn default() -> Self { JointType::Fixed }
}

/// Internal representation of a single widget entity.
/// Holds the cross-system handles and the widget's configuration.
#[derive(Debug)]
pub(crate) struct GhostEntity {
    pub id:           EntityId,
    pub taffy_node:   u64,              // taffy::NodeId (opaque)
    pub rapier_body:  u64,              // rapier2d::RigidBodyHandle (opaque)
    pub wgpu_index:   u32,              // index into SDF instance buffer
    pub physics:      PhysicsProps,
    pub style:        Style,
    pub layout_rect:  LayoutRect,       // last resolved Taffy output
    pub children:     Vec<EntityId>,
    pub parent:       Option<EntityId>,
    pub joint:        JointType,
    pub altitude:     f32,
}

/// The Ghost ECS — internal entity store.
/// pub(crate) only. The #[component] macro is the only public door.
pub(crate) struct GhostWorld {
    entities: SlotMap<EntityId, GhostEntity>,
}

impl GhostWorld {
    pub(crate) fn new() -> Self {
        Self { entities: SlotMap::with_key() }
    }

    pub(crate) fn spawn(&mut self, entity: GhostEntity) -> EntityId {
        self.entities.insert(entity)
    }

    pub(crate) fn despawn(&mut self, id: EntityId) -> Option<GhostEntity> {
        self.entities.remove(id)
    }

    pub(crate) fn get(&self, id: EntityId) -> Option<&GhostEntity> {
        self.entities.get(id)
    }

    pub(crate) fn get_mut(&mut self, id: EntityId) -> Option<&mut GhostEntity> {
        self.entities.get_mut(id)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &GhostEntity> {
        self.entities.values()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut GhostEntity> {
        self.entities.values_mut()
    }
}
