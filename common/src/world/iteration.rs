// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Data structures for representing ranges and iteration of blocks and chunks.

use super::ChunkCoordinate;
use itertools::{Itertools, Product};
use std::ops::Range;

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
