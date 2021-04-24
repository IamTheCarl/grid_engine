// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use derive_error::Error;
use itertools::{Itertools, Product};
use legion::World;
use nalgebra::Vector3;
use std::{collections::HashMap, ops::Range, time::Duration};

// pub mod inventory;
pub mod storage;
mod time;
pub use time::*;

// Names of files and folders in a world save.
// const TERRAIN_FOLDER: &str = "terrain";

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

/// Meta data used to describe a block.
// struct BlockData {
//     type_id: u16,
//     name: String,
// }

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

/// A tool to select a range of chunks (a big box)
pub struct ChunkRange {
    root_chunk: ChunkCoordinate,
    size: ChunkCoordinate, // Constructors must make sure this is never negative.
}

impl ChunkRange {
    /// Select a range of chunks using two corner points.
    pub fn from_end_points(first: ChunkCoordinate, second: ChunkCoordinate) -> ChunkRange {
        // Use the min values to find the root chunk.
        let root_chunk = first.inf(&second);

        // The size of the selection.
        let size = (first - second).abs();

        ChunkRange { root_chunk, size }
    }

    /// Get the two chunks most down-west-south and the chunk most up-east-north for this range.
    pub fn get_near_and_far(&self) -> (ChunkCoordinate, ChunkCoordinate) {
        (self.root_chunk, self.root_chunk + self.size)
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yxz(&self) -> ChunkIterator {
        let (near, far) = self.get_near_and_far();
        ChunkIterator {
            internal_iterator: (near.y..far.y).cartesian_product(near.x..far.x).cartesian_product(near.z..far.z),
            conversion_function: &|y, x, z| ChunkCoordinate::new(x, y, z),
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yzx(&self) -> ChunkIterator {
        let (near, far) = self.get_near_and_far();
        ChunkIterator {
            internal_iterator: (near.y..far.y).cartesian_product(near.z..far.z).cartesian_product(near.x..far.x),
            conversion_function: &|y, z, x| ChunkCoordinate::new(x, y, z),
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xyz(&self) -> ChunkIterator {
        let (near, far) = self.get_near_and_far();
        ChunkIterator {
            internal_iterator: (near.x..far.x).cartesian_product(near.y..far.y).cartesian_product(near.z..far.z),
            conversion_function: &|x, y, z| ChunkCoordinate::new(x, y, z),
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xzy(&self) -> ChunkIterator {
        let (near, far) = self.get_near_and_far();
        ChunkIterator {
            internal_iterator: (near.x..far.x).cartesian_product(near.z..far.z).cartesian_product(near.y..far.y),
            conversion_function: &|x, z, y| ChunkCoordinate::new(x, y, z),
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zxy(&self) -> ChunkIterator {
        let (near, far) = self.get_near_and_far();
        ChunkIterator {
            internal_iterator: (near.z..far.z).cartesian_product(near.x..far.x).cartesian_product(near.y..far.y),
            conversion_function: &|z, x, y| ChunkCoordinate::new(x, y, z),
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zyx(&self) -> ChunkIterator {
        let (near, far) = self.get_near_and_far();
        ChunkIterator {
            internal_iterator: (near.z..far.z).cartesian_product(near.y..far.y).cartesian_product(near.x..far.x),
            conversion_function: &|z, y, x| ChunkCoordinate::new(x, y, z),
        }
    }
}

/// An iterator for iterating over a range of chunks.
pub struct ChunkIterator {
    internal_iterator: Product<Product<Range<i16>, Range<i16>>, Range<i16>>,
    conversion_function: &'static dyn Fn(i16, i16, i16) -> ChunkCoordinate,
}

impl Iterator for ChunkIterator {
    type Item = ChunkCoordinate;
    fn next(&mut self) -> Option<ChunkCoordinate> {
        let next = self.internal_iterator.next();
        if let Some(((a, b), c)) = next {
            let conversion_function = self.conversion_function;
            Some(conversion_function(a, b, c))
        } else {
            None
        }
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
    chunk_provider: ChunkProvider,
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
    pub fn get_chunk(&self, index: &ChunkCoordinate) -> Option<&Chunk> {
        self.terrain_chunks.get(index)
    }

    /// Get a chunk from its index.
    pub fn get_chunk_mut(&mut self, index: &ChunkCoordinate) -> Option<&mut Chunk> {
        self.terrain_chunks.get_mut(index)
    }

    /// Get a chunk. If it doesn't exist, it will be loaded or generated. In other words, you're guaranteed to always get a chunk.
    pub fn load_chunk(&mut self, index: ChunkCoordinate) -> &mut Chunk {
        let chunk_provider = &mut self.chunk_provider;
        self.terrain_chunks.entry(index).or_insert_with(move || chunk_provider(index))
    }

    /// Load many chunks in a range.
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
