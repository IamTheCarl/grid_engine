// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use legion::{system, Resources, Schedule, World};
use rapier3d::{
    dynamics::{CCDSolver, IntegrationParameters, JointSet, RigidBodySet},
    geometry::{BroadPhase, ColliderSet, NarrowPhase},
    pipeline::PhysicsPipeline,
};
use std::{collections::HashMap, time::Duration};

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
pub trait ChunkProvider<ChunkUserData> {
    /// Access the block registry.
    fn block_registry(&self) -> &BlockRegistry;

    /// Access the block registry mutably.
    fn block_registry_mut(&mut self) -> &mut BlockRegistry;

    /// When a chunk is created, it needs to be filled with blocks. An empty chunk will be provided
    /// to this method, and this method is to fill it with blocks.
    fn provide_chunk(&self, chunk: &mut Chunk<ChunkUserData>);
}

/// A world full of terrain and entities.
pub struct GridWorld<ChunkUserData> {
    time: WorldTime,
    terrain_chunks: HashMap<ChunkCoordinate, Chunk<ChunkUserData>>,
    ecs_world: World,
    ecs_schedule: Schedule,
    ecs_resources: Resources,
    chunk_provider: Box<dyn ChunkProvider<ChunkUserData>>,
}

/// Global constants in the physics engine that we can't just loosely toss into the ECS resources.
pub struct PhysicsGlobalConstants {
    gravity: PhysicsVector,
    integration_parameters: IntegrationParameters,
}

impl<ChunkUserData: Default> GridWorld<ChunkUserData> {
    /// Create a new world.
    pub fn new(chunk_provider: Box<dyn ChunkProvider<ChunkUserData>>) -> GridWorld<ChunkUserData> {
        let terrain_chunks = HashMap::new();
        let time = WorldTime::from_ms(0);

        let ecs_world = World::default();
        let ecs_schedule = Schedule::builder().add_system(ecs_physics_system()).build();
        let mut ecs_resources = Resources::default();

        ecs_resources.insert(PhysicsPipeline::new());
        ecs_resources.insert(PhysicsGlobalConstants {
            gravity: PhysicsVector::new(0.0, -9.81, 0.0),
            integration_parameters: IntegrationParameters::default(),
        });
        ecs_resources.insert(BroadPhase::new());
        ecs_resources.insert(NarrowPhase::new());
        ecs_resources.insert(RigidBodySet::new());
        ecs_resources.insert(ColliderSet::new());
        ecs_resources.insert(JointSet::new());
        ecs_resources.insert(CCDSolver::new());

        GridWorld { time, terrain_chunks, ecs_world, ecs_schedule, ecs_resources, chunk_provider }
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

        self.ecs_schedule.execute(&mut self.ecs_world, &mut self.ecs_resources);
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

    /// Grab the ECS resource set which contains things like the physics engine.
    #[inline]
    pub fn ecs_resources(&self) -> &Resources {
        &self.ecs_resources
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

    /// Grab the ECS resource set which contains things like the physics engine.
    #[inline]
    pub fn ecs_resources_mut(&mut self) -> &mut Resources {
        &mut self.ecs_resources
    }

    /// Get a chunk from its index.
    #[inline]
    pub fn get_chunk(&self, index: &ChunkCoordinate) -> Option<&Chunk<ChunkUserData>> {
        self.terrain_chunks.get(index)
    }

    /// Get a chunk from its index.
    #[inline]
    pub fn get_chunk_mut(&mut self, index: &ChunkCoordinate) -> Option<&mut Chunk<ChunkUserData>> {
        self.terrain_chunks.get_mut(index)
    }

    /// Get a chunk. If it doesn't exist, it will be loaded or generated. In other words, you're guaranteed to always get a chunk.
    #[inline]
    pub fn load_chunk(&mut self, index: ChunkCoordinate) -> &mut Chunk<ChunkUserData> {
        let chunk_provider = &mut self.chunk_provider;
        self.terrain_chunks.entry(index).or_insert_with(|| {
            let mut chunk = Chunk::new(index, ChunkUserData::default());
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
fn ecs_physics(
    #[resource] physics_pipeline: &mut PhysicsPipeline, #[resource] constants: &PhysicsGlobalConstants,
    #[resource] broad_phase: &mut BroadPhase, #[resource] narrow_phase: &mut NarrowPhase,
    #[resource] rigid_bodies: &mut RigidBodySet, #[resource] colliders: &mut ColliderSet, #[resource] joints: &mut JointSet,
    #[resource] ccd_solver: &mut CCDSolver,
) {
    physics_pipeline.step(
        &constants.gravity,
        &constants.integration_parameters,
        broad_phase,
        narrow_phase,
        rigid_bodies,
        colliders,
        joints,
        ccd_solver,
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

        let _world: GridWorld<()> = GridWorld::new(chunk_provider);
    }

    /// Generate some chunks.
    #[test]
    fn generate_chunks() {
        let block_registry = BlockRegistry::new();
        let mut chunk_provider = chunk_providers::RAMWorld::new(block_registry);

        let abstract_flat_world = chunk_providers::AbstractFlatWorld::new();
        chunk_provider.add_generator(abstract_flat_world);

        let mut world: GridWorld<()> = GridWorld::new(chunk_provider);

        let abstract_block_id = world.block_registry().get_block_id_from_name("abstract_block").cloned();
        assert!(abstract_block_id.is_some());

        // Being at level 0, it should be filled with abstract blocks.
        let chunk = world.load_chunk(ChunkCoordinate::new(0, 0, 0));

        for block in chunk.iter_ideal(Chunk::<()>::range_all_blocks()) {
            assert_eq!(block, abstract_block_id);
        }

        // Being at level 1, it should be empty.
        let chunk = world.load_chunk(ChunkCoordinate::new(0, 1, 0));

        for block in chunk.iter_ideal(Chunk::<()>::range_all_blocks()) {
            assert_eq!(block, None);
        }
    }
}
