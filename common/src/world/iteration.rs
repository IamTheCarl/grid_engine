// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Data structures for representing ranges and iteration of blocks and chunks.

use super::{
    BlockID, Chunk, ChunkCoordinate, GlobalBlockCoordinate, GlobalBlockCoordinateEXT, GridWorld, LocalBlockCoordinate,
    LocalBlockCoordinateExt,
};
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

/// A selection of blocks within a single chunk.
pub struct LocalBlockRange {
    root_block: LocalBlockCoordinate,
    size: LocalBlockCoordinate, // Constructors must make sure this is never negative.
}

/// An iterator for iterating over a range of blocks within a chunk.
pub struct LocalBlockIterator<'chunk> {
    internal_iterator: Product<Product<Range<u8>, Range<u8>>, Range<u8>>,
    conversion_function: &'static dyn Fn(u8, u8, u8) -> LocalBlockCoordinate,
    chunk: &'chunk Chunk,
}

impl<'chunk> Iterator for LocalBlockIterator<'chunk> {
    type Item = Option<BlockID>;
    fn next(&mut self) -> Option<Option<BlockID>> {
        let next = self.internal_iterator.next();
        if let Some(((a, b), c)) = next {
            let conversion_function = self.conversion_function;
            let address = conversion_function(a, b, c);

            // Haha so yes, I'm using the function that asks you not to use it to iterate.
            // I said that in the documentation for two reasons.
            // First: so that people using this would write prettier code using iterators.
            // Second: Because I'm reserving the right to write more efficient iterators in the future.
            Some(self.chunk.get_single_block_local(address))
        } else {
            None
        }
    }
}

/// An iterator for iterating over a range of blocks within a chunk that you can modify.
pub struct LocalBlockIteratorMut<'chunk> {
    internal_iterator: Product<Product<Range<u8>, Range<u8>>, Range<u8>>,
    conversion_function: &'static dyn Fn(u8, u8, u8) -> LocalBlockCoordinate,
    chunk: &'chunk mut Chunk,
}

impl<'chunk> Iterator for LocalBlockIteratorMut<'chunk> {
    type Item = &'chunk mut Option<BlockID>;
    fn next(&mut self) -> Option<&'chunk mut Option<BlockID>> {
        let next = self.internal_iterator.next();
        if let Some(((a, b), c)) = next {
            let conversion_function = self.conversion_function;
            let address = conversion_function(a, b, c);

            // Haha so yes, I'm using the function that asks you not to use it to iterate.
            // See the non mutable iterator for details on that.

            // Yes, unsafe was needed here to make the lifetimes work. I can't prove to the borrow checker
            // that this iterator won't backup unexpectedly, so I have to ask it to trust me.
            let block = self.chunk.get_single_block_local_mut(address) as *mut _;

            Some(unsafe { &mut *block })
        } else {
            None
        }
    }
}

impl LocalBlockRange {
    /// Select a range of blocks using two corner points.
    /// Because it is possible to specify blocks outside the physically possible range of a chunk, this has error
    /// handling for block addresses out of range, specifically clearing the higher bits out of the valid address range.
    pub fn from_end_points(first: LocalBlockCoordinate, second: LocalBlockCoordinate) -> LocalBlockRange {
        // Clean up the vectors to make sure they're in a valid range.
        let first = first.validate();
        let second = second.validate();

        // Use the min values to find the root block.
        let root_block = first.inf(&second);

        // The size of the selection.
        // We need to get the difference between these two. It's easier to do that when they're in integer form.
        // I represent them as an i16, twice the size I need, because I just don't feel a need to deal with the
        // potential integer overflow.
        let size = (first.cast::<i16>() - second.cast::<i16>()).abs().map(|x| x as u8);

        LocalBlockRange { root_block, size }
    }

    /// Get the two chunks most down-west-south and the chunk most up-east-north for this range.
    pub fn get_near_and_far(&self) -> (LocalBlockCoordinate, LocalBlockCoordinate) {
        (self.root_block, self.root_block + self.size)
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yxz<'chunk>(&self, chunk: &'chunk Chunk) -> LocalBlockIterator<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIterator {
            internal_iterator: (near.y..far.y).cartesian_product(near.x..far.x).cartesian_product(near.z..far.z),
            conversion_function: &|y, x, z| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yzx<'chunk>(&self, chunk: &'chunk Chunk) -> LocalBlockIterator<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIterator {
            internal_iterator: (near.y..far.y).cartesian_product(near.z..far.z).cartesian_product(near.x..far.x),
            conversion_function: &|y, z, x| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xyz<'chunk>(&self, chunk: &'chunk Chunk) -> LocalBlockIterator<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIterator {
            internal_iterator: (near.x..far.x).cartesian_product(near.y..far.y).cartesian_product(near.z..far.z),
            conversion_function: &|x, y, z| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xzy<'chunk>(&self, chunk: &'chunk Chunk) -> LocalBlockIterator<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIterator {
            internal_iterator: (near.x..far.x).cartesian_product(near.z..far.z).cartesian_product(near.y..far.y),
            conversion_function: &|x, z, y| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zxy<'chunk>(&self, chunk: &'chunk Chunk) -> LocalBlockIterator<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIterator {
            internal_iterator: (near.z..far.z).cartesian_product(near.x..far.x).cartesian_product(near.y..far.y),
            conversion_function: &|z, x, y| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zyx<'chunk>(&self, chunk: &'chunk Chunk) -> LocalBlockIterator<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIterator {
            internal_iterator: (near.z..far.z).cartesian_product(near.y..far.y).cartesian_product(near.x..far.x),
            conversion_function: &|z, y, x| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yxz_mut<'chunk>(&self, chunk: &'chunk mut Chunk) -> LocalBlockIteratorMut<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIteratorMut {
            internal_iterator: (near.y..far.y).cartesian_product(near.x..far.x).cartesian_product(near.z..far.z),
            conversion_function: &|y, x, z| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yzx_mut<'chunk>(&self, chunk: &'chunk mut Chunk) -> LocalBlockIteratorMut<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIteratorMut {
            internal_iterator: (near.y..far.y).cartesian_product(near.z..far.z).cartesian_product(near.x..far.x),
            conversion_function: &|y, z, x| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xyz_mut<'chunk>(&self, chunk: &'chunk mut Chunk) -> LocalBlockIteratorMut<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIteratorMut {
            internal_iterator: (near.x..far.x).cartesian_product(near.y..far.y).cartesian_product(near.z..far.z),
            conversion_function: &|x, y, z| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xzy_mut<'chunk>(&self, chunk: &'chunk mut Chunk) -> LocalBlockIteratorMut<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIteratorMut {
            internal_iterator: (near.x..far.x).cartesian_product(near.z..far.z).cartesian_product(near.y..far.y),
            conversion_function: &|x, z, y| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zxy_mut<'chunk>(&self, chunk: &'chunk mut Chunk) -> LocalBlockIteratorMut<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIteratorMut {
            internal_iterator: (near.z..far.z).cartesian_product(near.x..far.x).cartesian_product(near.y..far.y),
            conversion_function: &|z, x, y| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zyx_mut<'chunk>(&self, chunk: &'chunk mut Chunk) -> LocalBlockIteratorMut<'chunk> {
        let (near, far) = self.get_near_and_far();
        LocalBlockIteratorMut {
            internal_iterator: (near.z..far.z).cartesian_product(near.y..far.y).cartesian_product(near.x..far.x),
            conversion_function: &|z, y, x| LocalBlockCoordinate::new(x, y, z),
            chunk,
        }
    }
}

/// A tool to select a range of blocks in global coordinate space.
pub struct GlobalBlockRange {
    root_block: GlobalBlockCoordinate,
    size: GlobalBlockCoordinate, // Constructors must make sure this is never negative.
}

// TODO make versions of these iterators that force the terrain to load if needed.

/// An iterator for iterating over a range of blocks.
pub struct GlobalBlockIterator<'world> {
    internal_iterator: Product<Product<Range<i64>, Range<i64>>, Range<i64>>,
    conversion_function: &'static dyn Fn(i64, i64, i64) -> GlobalBlockCoordinate,
    world: &'world GridWorld,
}

impl<'world> Iterator for GlobalBlockIterator<'world> {
    type Item = Option<BlockID>;
    fn next(&mut self) -> Option<Option<BlockID>> {
        let next = self.internal_iterator.next();
        if let Some(((a, b), c)) = next {
            let conversion_function = self.conversion_function;
            let address = conversion_function(a, b, c);

            // Extremely inefficient fetching of the chunk every time we increment.
            let chunk = self.world.get_chunk(&address.chunk_index())?;

            // Also pretty inefficient.
            Some(chunk.get_single_block_local(address.to_local_block_coordinate()))
        } else {
            None
        }
    }
}

/// An iterator for iterating over a range of blocks that you can modify.
pub struct GlobalBlockIteratorMut<'world> {
    internal_iterator: Product<Product<Range<i64>, Range<i64>>, Range<i64>>,
    conversion_function: &'static dyn Fn(i64, i64, i64) -> GlobalBlockCoordinate,
    world: &'world mut GridWorld,
}

impl<'chunk> Iterator for GlobalBlockIteratorMut<'chunk> {
    type Item = &'chunk mut Option<BlockID>;
    fn next(&mut self) -> Option<&'chunk mut Option<BlockID>> {
        let next = self.internal_iterator.next();
        if let Some(((a, b), c)) = next {
            let conversion_function = self.conversion_function;
            let address = conversion_function(a, b, c);

            // Extremely inefficient fetching of the chunk every time we increment.
            let chunk = self.world.get_chunk_mut(&address.chunk_index())?;
            let block = chunk.get_single_block_local_mut(address.to_local_block_coordinate()) as *mut _;

            Some(unsafe { &mut *block })
        } else {
            None
        }
    }
}

impl GlobalBlockRange {
    /// Select a range of chunks using two corner points.
    pub fn from_end_points(first: ChunkCoordinate, second: ChunkCoordinate) -> ChunkRange {
        // Use the min values to find the root chunk.
        let root_chunk = first.inf(&second);

        // The size of the selection.
        let size = (first - second).abs();

        ChunkRange { root_chunk, size }
    }

    /// Get the two chunks most down-west-south and the chunk most up-east-north for this range.
    pub fn get_near_and_far(&self) -> (GlobalBlockCoordinate, GlobalBlockCoordinate) {
        (self.root_block, self.root_block + self.size)
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yxz<'world>(&self, world: &'world GridWorld) -> GlobalBlockIterator<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIterator {
            internal_iterator: (near.y..far.y).cartesian_product(near.x..far.x).cartesian_product(near.z..far.z),
            conversion_function: &|y, x, z| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yzx<'world>(&self, world: &'world GridWorld) -> GlobalBlockIterator<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIterator {
            internal_iterator: (near.y..far.y).cartesian_product(near.z..far.z).cartesian_product(near.x..far.x),
            conversion_function: &|y, z, x| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xyz<'world>(&self, world: &'world GridWorld) -> GlobalBlockIterator<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIterator {
            internal_iterator: (near.x..far.x).cartesian_product(near.y..far.y).cartesian_product(near.z..far.z),
            conversion_function: &|x, y, z| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xzy<'world>(&self, world: &'world GridWorld) -> GlobalBlockIterator<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIterator {
            internal_iterator: (near.x..far.x).cartesian_product(near.z..far.z).cartesian_product(near.y..far.y),
            conversion_function: &|x, z, y| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zxy<'world>(&self, world: &'world GridWorld) -> GlobalBlockIterator<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIterator {
            internal_iterator: (near.z..far.z).cartesian_product(near.x..far.x).cartesian_product(near.y..far.y),
            conversion_function: &|z, x, y| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zyx<'world>(&self, world: &'world GridWorld) -> GlobalBlockIterator<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIterator {
            internal_iterator: (near.z..far.z).cartesian_product(near.y..far.y).cartesian_product(near.x..far.x),
            conversion_function: &|z, y, x| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yxz_mut<'world>(&self, world: &'world mut GridWorld) -> GlobalBlockIteratorMut<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIteratorMut {
            internal_iterator: (near.y..far.y).cartesian_product(near.x..far.x).cartesian_product(near.z..far.z),
            conversion_function: &|y, x, z| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_yzx_mut<'world>(&self, world: &'world mut GridWorld) -> GlobalBlockIteratorMut<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIteratorMut {
            internal_iterator: (near.y..far.y).cartesian_product(near.z..far.z).cartesian_product(near.x..far.x),
            conversion_function: &|y, z, x| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xyz_mut<'world>(&self, world: &'world mut GridWorld) -> GlobalBlockIteratorMut<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIteratorMut {
            internal_iterator: (near.x..far.x).cartesian_product(near.y..far.y).cartesian_product(near.z..far.z),
            conversion_function: &|x, y, z| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_xzy_mut<'world>(&self, world: &'world mut GridWorld) -> GlobalBlockIteratorMut<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIteratorMut {
            internal_iterator: (near.x..far.x).cartesian_product(near.z..far.z).cartesian_product(near.y..far.y),
            conversion_function: &|x, z, y| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zxy_mut<'world>(&self, world: &'world mut GridWorld) -> GlobalBlockIteratorMut<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIteratorMut {
            internal_iterator: (near.z..far.z).cartesian_product(near.x..far.x).cartesian_product(near.y..far.y),
            conversion_function: &|z, x, y| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }

    /// Get an iterator that iterates over the chunks in a cartesian manner.
    pub fn iter_zyx_mut<'world>(&self, world: &'world mut GridWorld) -> GlobalBlockIteratorMut<'world> {
        let (near, far) = self.get_near_and_far();
        GlobalBlockIteratorMut {
            internal_iterator: (near.z..far.z).cartesian_product(near.y..far.y).cartesian_product(near.x..far.x),
            conversion_function: &|z, y, x| GlobalBlockCoordinate::new(x, y, z),
            world,
        }
    }
}
