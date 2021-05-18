// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Components that can be used within the ECS.

use legion::Resources;
use rapier3d::{
    dynamics::{RigidBodyHandle, RigidBodySet},
    geometry::{Collider, ColliderHandle, ColliderSet},
};

/// A rigid body is part of the physics engine. It's a collection of shapes that make up a full object.
/// This component just references the rigid body within the physics engine.
pub struct RigidBody {
    handle: RigidBodyHandle,
}

impl RigidBody {
    /// Create a new rigid body component.
    pub fn new(resource_set: &mut Resources, rigid_body: rapier3d::dynamics::RigidBody) -> RigidBody {
        let mut rigid_bodies = resource_set.get_mut::<RigidBodySet>().expect("Failed to find rigid body set.");

        RigidBody { handle: rigid_bodies.insert(rigid_body) }
    }

    /// Add a collider (shape) to the rigid body.
    pub fn add_collider(&self, collider: Collider, resource_set: &mut Resources) -> ColliderHandle {
        let mut rigid_bodies = resource_set.get_mut::<RigidBodySet>().expect("Failed to find rigid body set.");
        let mut colliders = resource_set.get_mut::<ColliderSet>().expect("Failed to find collider set.");

        colliders.insert(collider, self.handle, &mut *rigid_bodies)
    }
}

#[cfg(test)]
mod test {
    // Import the world.
    use super::super::*;

    // Import ourselves.
    use super::*;

    use rapier3d::{dynamics::RigidBodyBuilder, geometry::ColliderBuilder};

    /// Create a world. Add a single rigid body. Tick the world once.
    #[test]
    fn rigid_body() {
        let block_registry = BlockRegistry::new();
        let mut chunk_provider = chunk_providers::RAMWorld::new(block_registry);

        let abstract_flat_world = chunk_providers::AbstractFlatWorld::new();
        chunk_provider.add_generator(abstract_flat_world);

        let mut world: GridWorld<()> = GridWorld::new(chunk_provider);

        let components = (RigidBody::new(world.ecs_resources_mut(), RigidBodyBuilder::new_dynamic().build()), ());
        world.ecs_world_mut().push(components);

        world.update(Duration::from_millis(10));
    }

    /// Create a world. Add a single rigid body. Give it a collider. Tick the world once.
    #[test]
    fn collider() {
        let block_registry = BlockRegistry::new();
        let mut chunk_provider = chunk_providers::RAMWorld::new(block_registry);

        let abstract_flat_world = chunk_providers::AbstractFlatWorld::new();
        chunk_provider.add_generator(abstract_flat_world);

        let mut world: GridWorld<()> = GridWorld::new(chunk_provider);

        let rigid_body = RigidBody::new(world.ecs_resources_mut(), RigidBodyBuilder::new_dynamic().build());
        rigid_body.add_collider(ColliderBuilder::ball(0.5f32).build(), world.ecs_resources_mut());
        let components = (rigid_body, ());
        world.ecs_world_mut().push(components);

        world.update(Duration::from_millis(10));
    }
}
