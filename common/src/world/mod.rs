// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use std::{collections::HashMap, path::Path};

pub mod inventory;
pub mod storage;
mod time;
pub use time::*;

// Names of files and folders in a world save.
const TERRAIN_FOLDER: &str = "terrain";

pub struct Chunk {
    storage: Option<Box<storage::ChunkData>>,
}

fn block_coordinate_to_chunk_coordinate(coordinate: (i64, i64, i64)) -> (i16, i16, i16) {
    (
        (coordinate.0 >> storage::BLOCK_ADDRESS_BITS) as i16,
        (coordinate.1 >> storage::BLOCK_ADDRESS_BITS) as i16,
        (coordinate.2 >> storage::BLOCK_ADDRESS_BITS) as i16,
    )
}

pub struct GridWorld {
    time: WorldTime,
    storage: storage::ChunkDiskStorage,
    terrain_chunks: HashMap<(i16, i16, i16), Chunk>,
}

impl GridWorld {
    /// Create a new world with local storage.
    pub fn new(folder: &Path) -> GridWorld {
        let storage = storage::ChunkDiskStorage::initialize(&folder.join(TERRAIN_FOLDER), 6);
        let terrain_chunks = HashMap::new();
        let time = WorldTime::from_ms(0);

        GridWorld { time, storage, terrain_chunks }
    }

    /// Update the entities of the world.
    pub fn update(&mut self) {}
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
