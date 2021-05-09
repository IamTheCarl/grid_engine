// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! All stuff relating to chunks, the big hunks of the world full of terrain.

use super::{
    coordinates::{ChunkCoordinate, LocalBlockCoordinate, LocalBlockCoordinateExt},
    storage, BlockID, LocalBlockIterator, LocalBlockIteratorMut, LocalBlockRange,
};
use derive_error::Error;
use std::num::NonZeroU16;

/// Error type for chunks.
#[derive(Debug, Error)]
pub enum ChunkError {
    /// An error for when you try to access something out of range.
    OutOfRange,
}

/// A chunk error type.
pub type ChunkResult<O> = std::result::Result<O, ChunkError>;

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
    pub fn direct_access(&self, index: usize) -> ChunkResult<Option<BlockID>> {
        let block_id = self.storage.get_data().get(index).ok_or(ChunkError::OutOfRange)?;

        Ok(if let Some(block_id) = NonZeroU16::new(*block_id) { Some(BlockID::new(block_id)) } else { None })
    }

    /// Used internally efficiently iterate the content of the chunk.
    /// You're best off not using this directly.
    #[inline]
    pub fn direct_access_mut(&mut self, index: usize) -> ChunkResult<&mut Option<BlockID>> {
        let block_id = self.storage.get_data_mut().get_mut(index).ok_or(ChunkError::OutOfRange)?;

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
