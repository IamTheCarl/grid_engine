// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Vector types for block and chunk coordinates.

use super::storage;
use nalgebra::Vector3;

/// Type for a chunk's coordinates in chunk space.
pub type ChunkCoordinate = Vector3<i16>;

/// Type for a block's coordinates in chunk local space.
pub type LocalBlockCoordinate = Vector3<u8>;

/// Type for a block's coordinates in global space.
pub type GlobalBlockCoordinate = Vector3<i64>;

/// Adds the ability to convert the chunk coordinate to a block global space coordinate.
pub trait ChunkCoordinateEXT {
    /// Get the coordinate of local block 0x0x0 in global space.
    fn to_block_coordinate(&self) -> GlobalBlockCoordinate;
}

impl ChunkCoordinateEXT for ChunkCoordinate {
    fn to_block_coordinate(&self) -> GlobalBlockCoordinate {
        self.map(|v| (v as i64) << storage::NUM_BLOCK_ADDRESS_BITS)
    }
}

/// Add additional features to the vector type for global block coordinates.
pub trait GlobalBlockCoordinateEXT {
    /// Get the position of the block within its chunk.
    fn to_local_block_coordinate(&self) -> LocalBlockCoordinate;

    /// Get the index of the chunk this block is in.
    fn chunk_index(&self) -> ChunkCoordinate;
}

impl GlobalBlockCoordinateEXT for GlobalBlockCoordinate {
    fn to_local_block_coordinate(&self) -> LocalBlockCoordinate {
        self.map(|v| (v & storage::LOCAL_BLOCK_COORDINATE_BITS) as u8)
    }

    fn chunk_index(&self) -> ChunkCoordinate {
        self.map(|v| (v >> storage::NUM_BLOCK_ADDRESS_BITS) as i16)
    }
}

/// Additional features added to the vector type for local block coordinates.
pub trait LocalBlockCoordinateExt {
    /// Get the position of the block in global space, by offsetting it by the block position of its chunk.
    fn to_global_block_coordinate(&self, chunk_position: ChunkCoordinate) -> GlobalBlockCoordinate;

    /// Validate that the coordinates are within the valid range for a chunk. If the higher bits putting them
    /// outside the range of a chunk are set, they will be cleared and that value returned.
    fn validate(&self) -> Self;
}

impl LocalBlockCoordinateExt for LocalBlockCoordinate {
    fn to_global_block_coordinate(&self, chunk_position: ChunkCoordinate) -> GlobalBlockCoordinate {
        self.cast() + chunk_position.to_block_coordinate()
    }

    fn validate(&self) -> LocalBlockCoordinate {
        self.map(|v| v & (storage::LOCAL_BLOCK_COORDINATE_BITS as u8))
    }
}
