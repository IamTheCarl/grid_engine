// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Components that can be used within the ECS.

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
    /// Add a collider (shape) to the rigid body.
    pub fn add_collider(
        &self, collider: Collider, rigid_body_set: &mut RigidBodySet, collider_set: &mut ColliderSet,
    ) -> ColliderHandle {
        collider_set.insert(collider, self.handle, rigid_body_set)
    }
}
