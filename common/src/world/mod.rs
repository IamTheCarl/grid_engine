// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use derive_error::Error;
use legion::World;
use serde::{Deserialize, Serialize};
use std::{
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    collections::HashMap,
    fmt,
    num::NonZeroU16,
    time::Duration,
};

// pub mod inventory;
mod coordinates;
mod iteration;
pub use coordinates::*;
pub mod storage;
mod time;
pub use iteration::*;
pub use time::*;
pub mod chunk_providers;

// Names of files and folders in a world save.
// const TERRAIN_FOLDER: &str = "terrain";

/// A registry of data about blocks.
/// Adding blocks to this structure is a simple process, but removing blocks require you go through all of the world's
/// chunks and update them to be compatible with the new registry. Currently, this is not supported.
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
    /// Construct a new block registry.
    pub fn new() -> BlockRegistry {
        BlockRegistry { block_data: Vec::new(), block_ids: HashMap::new() }
    }

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
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

    /// Get a single block from the chunk.
    /// Do NOT use this to iterate. Use the proper iterators to do so.
    /// This will chop off out of range bits for coordinates extending beyond chunk bounds.
    #[inline]
    pub fn get_single_block_local(&self, location: LocalBlockCoordinate) -> Option<BlockID> {
        let location = location.validate();
        self.direct_access(
            location.x as usize
                + location.y as usize * storage::CHUNK_DIAMETER
                + location.z as usize * storage::CHUNK_DIAMETER * storage::CHUNK_DIAMETER,
        )
        .expect("Local block index out of bounds.")
    }

    /// Get a single block from the chunk.
    /// Do NOT use this to iterate. Use the proper iterators to do so.
    /// This will chop off out of range bits for coordinates extending beyond chunk bounds.
    #[inline]
    pub fn get_single_block_local_mut(&mut self, location: LocalBlockCoordinate) -> &mut Option<BlockID> {
        let location = location.validate();
        self.direct_access_mut(
            location.x as usize
                + location.y as usize * storage::CHUNK_DIAMETER
                + location.z as usize * storage::CHUNK_DIAMETER * storage::CHUNK_DIAMETER,
        )
        .expect("Local block index out of bounds.")
    }

    /// Used internally efficiently iterate the content of the chunk.
    /// You're best off not using this directly.
    #[inline]
    pub fn direct_access(&self, index: usize) -> WorldResult<Option<BlockID>> {
        let block_id = self.storage.get_data().get(index).ok_or(WorldError::OutOfRange)?;

        Ok(if let Some(block_id) = NonZeroU16::new(*block_id) { Some(BlockID::new(block_id)) } else { None })
    }

    /// Used internally efficiently iterate the content of the chunk.
    /// You're best off not using this directly.
    #[inline]
    pub fn direct_access_mut(&mut self, index: usize) -> WorldResult<&mut Option<BlockID>> {
        let block_id = self.storage.get_data_mut().get_mut(index).ok_or(WorldError::OutOfRange)?;

        // We have to transmute this to keep it a reference. It should be safe since an Option<BlockID>
        // is just a normal u16 where 0 represents none.
        Ok(unsafe { std::mem::transmute(block_id) })
    }

    /// An ideal iterator for the chunk. This iterates in what is currently the most efficient way to iterate this chunk.
    /// The order in which blocks are iterated is subject to change at random, and may even be different each time you
    /// call this function.
    #[inline]
    pub fn iter_ideal(&self, range: LocalBlockRange) -> LocalBlockIterator {
        range.iter_xyz(self)
    }

    /// An ideal iterator for the chunk. This iterates in what is currently the most efficient way to iterate this chunk.
    /// The order in which blocks are iterated is subject to change at random, and may even be different each time you
    /// call this function.
    #[inline]
    pub fn iter_ideal_mut(&mut self, range: LocalBlockRange) -> LocalBlockIteratorMut {
        range.iter_xyz_mut(self)
    }

    /// A range for all blocks in the chunk.
    /// This is just nice for making code more readable.
    #[inline]
    pub fn range_all_blocks() -> LocalBlockRange {
        LocalBlockRange::from_end_points(
            LocalBlockCoordinate::new(0, 0, 0),
            LocalBlockCoordinate::new(
                storage::CHUNK_DIAMETER as u8,
                storage::CHUNK_DIAMETER as u8,
                storage::CHUNK_DIAMETER as u8,
            ),
        )
    }
}

/// Error type for the world.
#[derive(Debug, Error)]
pub enum WorldError {
    /// An error for when you try to access something out of range.
    OutOfRange,
}

/// A world error type.
pub type WorldResult<O> = std::result::Result<O, WorldError>;

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

    /// Get the world block registry.
    #[inline]
    pub fn block_registry(&self) -> &BlockRegistry {
        self.chunk_provider.block_registry()
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
    use super::*;
    // use inventory::*;
    // use tempfile::tempdir;

    /// Transmutation of block IDs is kind of a hack I had to use to make the direct_access_mut function on chunks work correctly.
    /// Since it is *possible* for that behavior to break in the future, this test is here to point out the issue quickly.
    #[test]
    fn block_id_transmutation() {
        let block_id = Some(BlockID { id: NonZeroU16::new(5).unwrap() });
        assert_eq!(*unsafe { std::mem::transmute::<&Option<BlockID>, &u16>(&block_id) }, 5u16);

        let block_id = None;
        assert_eq!(*unsafe { std::mem::transmute::<&Option<BlockID>, &u16>(&block_id) }, 0u16);
    }

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
