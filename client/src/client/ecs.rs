// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Client specific components and systems.

use common::world::components::RigidBody;
use legion::{system, world::Entry, Entity, World};
use nalgebra::{Isometry3, Perspective3};

/// An entity with both a rigid body to provide the Isometry3 *and* this component can be used as a camera.
/// If the entity is missing its rigid body, this component is ignored.
pub struct CameraComponent {
    perspective: Perspective3<f32>,
}
