// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Vector types for block and chunk coordinates.

use super::storage;
use nalgebra::Vector3;

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
