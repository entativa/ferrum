//! world.rs — The Ferrum physics world, backed by Rapier2D.

use std::collections::HashMap;
use fe_ui_core::{EntityId, physics::PhysicsProps, types::{LayoutRect, Transform2D, Vec2}};

pub struct PhysicsWorld {
    config: WorldConfig,
}

pub struct WorldConfig {
    pub gravity:           Vec2,
    pub air_density:       f32,
    pub pixels_per_meter:  f32,
    pub physics_hz:        f32,
    pub substeps:          u32,
    pub solver_iterations: u32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            gravity:           Vec2::new(0.0, 980.0),
            air_density:       0.1,
            pixels_per_meter:  100.0,
            physics_hz:        240.0,
            substeps:          4,
            solver_iterations: 8,
        }
    }
}

impl PhysicsWorld {
    pub fn new(config: WorldConfig) -> Self { Self { config } }

    pub fn spawn_body(&mut self, _id: EntityId, _props: &PhysicsProps) {}
    pub fn despawn_body(&mut self, _id: EntityId) {}
    pub fn update_spring_targets(&mut self, _targets: &HashMap<EntityId, LayoutRect>) {}
    pub fn apply_impulse(&mut self, _id: EntityId, _impulse: Vec2) {}

    pub fn step(&mut self) -> HashMap<EntityId, (Transform2D, Vec2)> {
        HashMap::new()
    }
}
