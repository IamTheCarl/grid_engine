// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use derive_error::Error;
use legion::World;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, time::Duration};

// pub mod inventory;
mod coordinates;
mod iteration;
pub use coordinates::*;
pub mod storage;
mod time;
pub use iteration::*;
pub use time::*;

// Names of files and folders in a world save.
// const TERRAIN_FOLDER: &str = "terrain";

/// Meta data used to describe a block.
/// Eventually more data should be associated, such as what happens when you break it, does it give off light? How heavy is it?
#[derive(Serialize, Deserialize)]
pub struct BlockData {
    name: String,
    display_text: String, // TODO grab this from a translation table?
}

impl fmt::Display for BlockData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        // Just show the displayed name.
        write!(formatter, "{}", &self.display_text)
    }
}

/// A registry of data about blocks.
/// Adding blocks to this structure is a simple process, but removing blocks require you go through all of the world's
/// chunks and update them to be compatible with the new registry. Currently, this is not supported.
#[derive(Serialize, Deserialize)]
pub struct BlockRegistry {
    block_data: Vec<BlockData>,
}

impl BlockRegistry {
    /// Add a block to the block registry.
    pub fn add_block(&mut self, name: String, display_text: String) {
        self.block_data.push(BlockData { name, display_text })
    }

    /// Get a block's data from its ID.
    /// Will panic if you provide an invalid ID!
    pub fn get_block(&self, id: u16) -> &BlockData {
        &self.block_data[id as usize]
    }
}

/// A chunk of the world's terrain.
pub struct Chunk {
    storage: Box<storage::ChunkData>,
}

impl Chunk {
    /// Get the index of the chunk.
    pub fn index(&self) -> ChunkCoordinate {
        self.storage.get_index()
    }
}

/// Error type for the world.
#[derive(Debug, Error)]
pub enum WorldError {
    /// There are no errors yet so this is just a place holder so this will compile.
    PlaceHolder,
}

/// A world error type.
pub type WorldResult<O> = std::result::Result<O, WorldError>;

/// A world full of terrain and entities.
pub struct GridWorld<ChunkProvider>
where
    ChunkProvider: Fn(ChunkCoordinate) -> Chunk,
{
    time: WorldTime,
    terrain_chunks: HashMap<ChunkCoordinate, Chunk>,
    entities: World,
    chunk_provider: ChunkProvider, // TODO use a trait and allocate on the heap instead.
}

impl<ChunkProvider> GridWorld<ChunkProvider>
where
    ChunkProvider: Fn(ChunkCoordinate) -> Chunk,
{
    /// Create a new world with local storage.
    pub fn new(chunk_provider: ChunkProvider) -> GridWorld<ChunkProvider> {
        let terrain_chunks = HashMap::new();
        let time = WorldTime::from_ms(0);
        let entities = World::default();

        GridWorld { time, terrain_chunks, entities, chunk_provider }
    }

    /// Update the entities of the world.
    pub fn update(&mut self, time_delta: Duration) {
        self.time += time_delta;
    }

    /// Get the world time.
    #[inline]
    pub fn time(&self) -> WorldTime {
        self.time
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
        self.terrain_chunks.entry(index).or_insert_with(move || chunk_provider(index))
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
