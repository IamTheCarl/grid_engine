//! The world and all its entities.

use specs::{World, WorldExt, DispatcherBuilder};

use crate::physics;

/// Creates the world and its associated dispatcher.
pub fn create_world<'a, 'b>() -> (World, DispatcherBuilder<'a, 'b>) {

    let world = World::new();
    let dispatcher = DispatcherBuilder::new();

    // Add physics stuff.
    let dispatcher = physics::add_systems(dispatcher);

    (world, dispatcher)
}