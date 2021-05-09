// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Stuff relating to terrain blocks.

use derive_error::Error;
use serde::{Deserialize, Serialize};
use std::{
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    collections::HashMap,
    fmt,
    num::NonZeroU16,
};

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

#[cfg(test)]
mod test {
    use super::*;
    /// Transmutation of block IDs is kind of a hack I had to use to make the direct_access_mut function on chunks work correctly.
    /// Since it is *possible* for that behavior to break in the future, this test is here to point out the issue quickly.
    #[test]
    fn block_id_transmutation() {
        let block_id = Some(BlockID { id: NonZeroU16::new(5).unwrap() });
        assert_eq!(*unsafe { std::mem::transmute::<&Option<BlockID>, &u16>(&block_id) }, 5u16);

        let block_id = None;
        assert_eq!(*unsafe { std::mem::transmute::<&Option<BlockID>, &u16>(&block_id) }, 0u16);
    }
}
