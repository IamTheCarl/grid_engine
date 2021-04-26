// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use derive_error::Error;
use legion::World;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, num::NonZeroU16, time::Duration};

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

/// A registry of data about blocks.
/// Adding blocks to this structure is a simple process, but removing blocks require you go through all of the world's
/// chunks and update them to be compatible with the new registry. Currently, this is not supported.
#[derive(Serialize, Deserialize)]
pub struct BlockRegistry {
    block_data: Vec<BlockData>,
    block_ids: HashMap<String, BlockID>,
}

/// Errors revolving around registries.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// This error happens if you attempt to add an item to the registry with the same key as an item
    /// already in the registry.
    KeyAlreadyExists,
}

/// Meta data used to describe a block.
/// Eventually more data should be associated, such as what happens when you break it, does it give off light? How heavy is it?
#[derive(Serialize, Deserialize)]
pub struct BlockData {
    name: String,
    id: BlockID,
    display_text: String, // TODO grab this from a translation table?
}

impl fmt::Display for BlockData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        // Just show the displayed name.
        write!(formatter, "{}", &self.display_text)
    }
}

type RegistryResult<O> = std::result::Result<O, RegistryError>;

impl BlockRegistry {
    /// Add a block to the block registry.
    pub fn add_block(&mut self, name: String, display_text: String) -> RegistryResult<()> {
        // We offset the block ID by 1 to make sure it is non-zero when the array index is zero.
        if !self.block_ids.contains_key(&name) {
            let id = BlockID::new(NonZeroU16::new((self.block_data.len() + 1) as u16).expect("Generated invalid block ID."));

            self.block_ids.insert(name.clone(), id);
            self.block_data.push(BlockData { name, id, display_text });

            Ok(())
        } else {
            Err(RegistryError::KeyAlreadyExists)
        }
    }

    /// Get a block's data from its ID.
    #[inline]
    pub fn get_block_data_from_id(&self, id: BlockID) -> Option<&BlockData> {
        // We subtract one because that fits into our block data range.
        self.block_data.get((id.id.get() - 1) as usize)
    }

    /// Get the ID of a block from its name.
    #[inline]
    pub fn get_block_id_from_name(&self, name: &str) -> Option<&BlockID> {
        self.block_ids.get(name)
    }

    /// Get a blocks data from its name.
    #[inline]
    pub fn get_block_data_from_name(&self, name: &str) -> Option<&BlockData> {
        let id = self.get_block_id_from_name(name)?;
        self.get_block_data_from_id(*id)
    }

    /// Get the number of different types of blocks.
    #[inline]
    pub fn num_block_types(&self) -> u16 {
        self.block_data.len() as u16
    }
}

/// Represents the ID of a single block in a terrain chunk.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct BlockID {
    id: NonZeroU16,
}

impl BlockID {
    /// Create a new block directly from a non-zero u16.
    /// This is generally not a good idea to do unless you're doing something unusual like loading terrain
    /// from a file. For the most part you should use block IDs you got from the block registry.
    pub fn new(id: NonZeroU16) -> BlockID {
        BlockID { id }
    }
}

/// A chunk of the world's terrain.
pub struct Chunk {
    storage: Box<storage::ChunkData>,
}

impl Chunk {
    /// Create a new, blank chunk.
    pub fn new(location: ChunkCoordinate) -> Chunk {
        Chunk { storage: storage::ChunkData::create(location) }
    }

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

/// An object that provides terrain chunks with their block content.
pub trait ChunkProvider {
    /// To put blocks into a chunk, you will need to get the block IDs ahead of time.
    /// This function lets you do so.
    fn collect_block_ids(&mut self, registry: &BlockRegistry);

    /// When a chunk is created, it needs to be filled with blocks. An empty chunk will be provided
    /// to this method, and this method is to fill it with blocks.
    fn provide_chunk(&self, chunk: &mut Chunk);
}

/// A world full of terrain and entities.
pub struct GridWorld {
    time: WorldTime,
    terrain_chunks: HashMap<ChunkCoordinate, Chunk>,
    ecs: World,
    chunk_provider: Box<dyn ChunkProvider>,
}

impl GridWorld {
    /// Create a new world with local storage.
    pub fn new(chunk_provider: Box<dyn ChunkProvider>) -> GridWorld {
        let terrain_chunks = HashMap::new();
        let time = WorldTime::from_ms(0);
        let ecs = World::default();

        GridWorld { time, terrain_chunks, ecs, chunk_provider }
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

    /// Grab the ECS for manipulating entities.
    #[inline]
    pub fn ecs(&self) -> &World {
        &self.ecs
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
