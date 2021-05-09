// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use legion::{system, Schedule, World};
use rapier3d::{
    dynamics::{CCDSolver, IntegrationParameters, JointSet, RigidBodySet},
    geometry::{BroadPhase, ColliderSet, NarrowPhase},
    pipeline::PhysicsPipeline,
};
use std::{collections::HashMap, time::Duration};

// pub mod inventory;
mod coordinates;
mod iteration;
pub use coordinates::*;
pub mod storage;
mod time;
pub use iteration::*;
pub use time::*;
pub mod chunk_providers;
pub mod components;

mod blocks;
pub use blocks::*;

mod chunk;
pub use chunk::*;

// Names of files and folders in a world save.
// const TERRAIN_FOLDER: &str = "terrain";

/// An object that provides terrain chunks with their block content.
pub trait ChunkProvider {
    /// Access the block registry.
    fn block_registry(&self) -> &BlockRegistry;

    /// Access the block registry mutably.
    fn block_registry_mut(&mut self) -> &mut BlockRegistry;

    /// When a chunk is created, it needs to be filled with blocks. An empty chunk will be provided
    /// to this method, and this method is to fill it with blocks.
    fn provide_chunk(&self, chunk: &mut Chunk);
}

/// A struct that contains everything we need for physics processing.
/// This just makes life easier when working with the ECS later.
struct WorldPhysics {
    physics_pipeline: PhysicsPipeline,
    gravity: PhysicsVector,
    physics_integration_parameters: IntegrationParameters,
    physics_broad_phase: BroadPhase,
    physics_narrow_phase: NarrowPhase,
    rigid_bodies: RigidBodySet,
    colliders: ColliderSet,
    physics_joints: JointSet,
    ccd_solver: CCDSolver,
}

/// A world full of terrain and entities.
pub struct GridWorld {
    time: WorldTime,
    terrain_chunks: HashMap<ChunkCoordinate, Chunk>,
    ecs_world: World,
    ecs_schedule: Schedule,
    chunk_provider: Box<dyn ChunkProvider>,
    physics: WorldPhysics,
}

impl GridWorld {
    /// Create a new world with local storage.
    pub fn new(chunk_provider: Box<dyn ChunkProvider>) -> GridWorld {
        let terrain_chunks = HashMap::new();
        let time = WorldTime::from_ms(0);
        let ecs_world = World::default();
        let ecs_schedule = Schedule::builder().add_system(ecs_physics_system()).build();

        let physics = WorldPhysics {
            physics_pipeline: PhysicsPipeline::new(),
            gravity: PhysicsVector::new(0.0, -9.81, 0.0),
            physics_integration_parameters: IntegrationParameters::default(),
            physics_broad_phase: BroadPhase::new(),
            physics_narrow_phase: NarrowPhase::new(),
            rigid_bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            physics_joints: JointSet::new(),
            ccd_solver: CCDSolver::new(),
        };

        GridWorld { time, terrain_chunks, ecs_world, ecs_schedule, chunk_provider, physics }
    }

    /// Get the world block registry.
    #[inline]
    pub fn block_registry(&self) -> &BlockRegistry {
        self.chunk_provider.block_registry()
    }

    /// Update the entities of the world.
    pub fn update(&mut self, time_delta: Duration) {
        // Update the time.
        self.time += time_delta;
    }

    /// Get the world time.
    #[inline]
    pub fn time(&self) -> WorldTime {
        self.time
    }

    /// Grab the ECS for manipulating entities.
    #[inline]
    pub fn ecs_world(&self) -> &World {
        &self.ecs_world
    }

    /// Grab the ECS schedule for manipulating systems.
    #[inline]
    pub fn ecs_schedule(&self) -> &Schedule {
        &self.ecs_schedule
    }

    /// Grab the ECS for manipulating entities.
    #[inline]
    pub fn ecs_world_mut(&mut self) -> &mut World {
        &mut self.ecs_world
    }

    /// Grab the ECS schedule for manipulating systems.
    #[inline]
    pub fn ecs_schedule_mut(&mut self) -> &mut Schedule {
        &mut self.ecs_schedule
    }

    /// Get a chunk from its index.
    #[inline]
    pub fn get_chunk(&self, index: &ChunkCoordinate) -> Option<&Chunk> {
        self.terrain_chunks.get(index)
    }

    /// Get a chunk from its index.
    #[inline]
    pub fn get_chunk_mut(&mut self, index: &ChunkCoordinate) -> Option<&mut Chunk> {
        self.terrain_chunks.get_mut(index)
    }

    /// Get a chunk. If it doesn't exist, it will be loaded or generated. In other words, you're guaranteed to always get a chunk.
    #[inline]
    pub fn load_chunk(&mut self, index: ChunkCoordinate) -> &mut Chunk {
        let chunk_provider = &mut self.chunk_provider;
        self.terrain_chunks.entry(index).or_insert_with(|| {
            let mut chunk = Chunk::new(index);
            chunk_provider.provide_chunk(&mut chunk);

            chunk
        })
    }

    /// Load many chunks in a range.
    #[inline]
    pub fn load_chunk_range(&mut self, range: ChunkRange) {
        // TODO it would be nice if we could make this run in parallel.
        for chunk_index in range.iter_xyz() {
            self.load_chunk(chunk_index);
        }
    }
}

// Next comes a bunch of systems used in the ECS.

/// The physics system. It will update the world's physics. That's it.
#[system]
fn ecs_physics(#[resource] physics: &mut WorldPhysics) {
    physics.physics_pipeline.step(
        &physics.gravity,
        &physics.physics_integration_parameters,
        &mut physics.physics_broad_phase,
        &mut physics.physics_narrow_phase,
        &mut physics.rigid_bodies,
        &mut physics.colliders,
        &mut physics.physics_joints,
        &mut physics.ccd_solver,
        &(),
        &(),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    /// Create an abstract RAM world, just to make sure that works.
    #[test]
    fn new_world_abstract_ram() {
        let block_registry = BlockRegistry::new();
        let mut chunk_provider = chunk_providers::RAMWorld::new(block_registry);

        let abstract_flat_world = chunk_providers::AbstractFlatWorld::new();
        chunk_provider.add_generator(abstract_flat_world);

        let _world = GridWorld::new(chunk_provider);
    }

    /// Generate some chunks.
    #[test]
    fn generate_chunks() {
        let block_registry = BlockRegistry::new();
        let mut chunk_provider = chunk_providers::RAMWorld::new(block_registry);

        let abstract_flat_world = chunk_providers::AbstractFlatWorld::new();
        chunk_provider.add_generator(abstract_flat_world);

        let mut world = GridWorld::new(chunk_provider);

        let abstract_block_id = world.block_registry().get_block_id_from_name("abstract_block").cloned();
        assert!(abstract_block_id.is_some());

        // Being at level 0, it should be filled with abstract blocks.
        let chunk = world.load_chunk(ChunkCoordinate::new(0, 0, 0));

        for block in chunk.iter_ideal(Chunk::range_all_blocks()) {
            assert_eq!(block, abstract_block_id);
        }

        // Being at level 1, it should be empty.
        let chunk = world.load_chunk(ChunkCoordinate::new(0, 1, 0));

        for block in chunk.iter_ideal(Chunk::range_all_blocks()) {
            assert_eq!(block, None);
        }
    }
}
