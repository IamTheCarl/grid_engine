// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use legion::World;
use nalgebra::Vector3;
use std::{collections::HashMap, path::Path, time::Duration};

// pub mod inventory;
pub mod storage;
mod time;
pub use time::*;

// Names of files and folders in a world save.
const TERRAIN_FOLDER: &str = "terrain";

/// Type for a chunk's coordinates in chunk space.
pub type ChunkCoordinate = Vector3<i16>;

/// Type for a block's coordinates in chunk local space.
pub type LocalBlockCoordinate = Vector3<i8>;

/// Type for a block's coordinates in global space.
pub type GlobalBlockCoordinate = Vector3<i64>;

/// Adds the ability to convert the chunk coordinate to a block global space coordinate.
trait ChunkCoordinateEXT {
    /// Get the coordinate of local block 0x0x0 in global space.
    fn to_block_coordinate(&self) -> GlobalBlockCoordinate;
}

impl ChunkCoordinateEXT for ChunkCoordinate {
    fn to_block_coordinate(&self) -> GlobalBlockCoordinate {
        self.map(|v| (v as i64) << storage::BLOCK_ADDRESS_BITS)
    }
}

trait GlobalBlockCoordinateEXT {
    /// Get the position of the block within its chunk.
    fn to_local_block_coordinate(&self) -> LocalBlockCoordinate;
}

impl GlobalBlockCoordinateEXT for GlobalBlockCoordinate {
    fn to_local_block_coordinate(&self) -> LocalBlockCoordinate {
        self.map(|v| (v & storage::BLOCK_COORDINATE_BITS) as i8)
    }
}

trait LocalBlockCoordinateExt {
    /// Get the position of the block in global space, by offsetting it by the block position of its chunk.
    fn to_global_block_coordinate(&self, chunk_position: ChunkCoordinate) -> GlobalBlockCoordinate;
}

impl LocalBlockCoordinateExt for LocalBlockCoordinate {
    fn to_global_block_coordinate(&self, chunk_position: ChunkCoordinate) -> GlobalBlockCoordinate {
        self.cast() + chunk_position.to_block_coordinate()
    }
}

/// A chunk of the world's terrain.
pub struct Chunk {
    storage: Option<Box<storage::ChunkData>>,
}

impl Chunk {}

/// A world full of terrain and entities.
pub struct GridWorld {
    time: WorldTime,
    storage: storage::ChunkDiskStorage,
    terrain_chunks: HashMap<(i16, i16, i16), Chunk>,
    entities: World,
}

impl GridWorld {
    /// Create a new world with local storage.
    pub fn new(folder: &Path) -> GridWorld {
        let storage = storage::ChunkDiskStorage::initialize(&folder.join(TERRAIN_FOLDER), 6);
        let terrain_chunks = HashMap::new();
        let time = WorldTime::from_ms(0);
        let entities = World::default();

        GridWorld { time, storage, terrain_chunks, entities }
    }

    /// Update the entities of the world.
    pub fn update(&mut self, time_delta: Duration) {
        self.time += time_delta;
    }

    /// Get the world time.
    pub fn time(&self) -> WorldTime {
        self.time
    }

    pub fn render_terrain(&self) {
        // TODO
    }
}

#[cfg(test)]
mod test {
    // use super::*;
    // use inventory::*;
    // use tempfile::tempdir;

    // /// Build an entity with no components.
    // #[test]
    // fn build_empty_entity() {
    //     let folder = tempdir().unwrap();

    //     let mut world = GridWorld::new(folder.path());
    //     let _id = world.create_entity(HashMap::new());
    // }

    // /// Build an entity with a single component (we happen to use the inventory component)
    // #[test]
    // fn build_entity_with_component() {
    //     let folder = tempdir().unwrap();

    //     let mut world = GridWorld::new(folder.path());
    //     let mut components: HashMap<String, Box<dyn Component>> = HashMap::new();

    //     components.insert(String::from("inventory"), Box::new(Inventory::infinite()));

    //     let _entity_id = world.create_entity(components);
    // }

    // /// Run a single event through a component.
    // #[test]
    // fn run_event() {
    //     let folder = tempdir().unwrap();

    //     let mut world = GridWorld::new(folder.path());
    //     let mut components: HashMap<String, Box<dyn Component>> = HashMap::new();

    //     components.insert(String::from("inventory"), Box::new(Inventory::infinite()));

    //     let entity_id = world.create_entity(components);

    //     let mut material_registry = MaterialRegistry::new();
    //     material_registry.register_material(String::from("obamium"), 4.2);
    //     let material_registry = material_registry; // Re-define without mutability.

    //     let material_id = material_registry.get_material_id("obamium").unwrap();

    //     world
    //         .push_event(
    //             entity_id,
    //             None,
    //             String::from("inventory"),
    //             &MaterialEvent::Add { stack: MaterialStack::new(material_id, 15) },
    //         )
    //         .unwrap();

    //     let event_count = world.update();
    //     assert_eq!(event_count, 1, "Wrong number of events processed.");
    // }
}
