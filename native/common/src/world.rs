// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

// Expected size of a block on the hard drive.
const BLOCK_SIZE: u64 = 4096;

// Actual size of a node in the index file.
// It's big enough to store 2^16 file pointers, making it 512Kb in size.
const NODE_LENGTH: u64 = BLOCK_SIZE * 128;

// A chunk is 16x16x16 blocks in size, and a block consists of two bytes.
// That makes the chunk 8Kb in length.
const CHUNK_LENGTH: u64 = 16 * 16 * 16 * 2;

create_file_pointer_type!(NodePointer);
create_file_pointer_type!(ChunkKey);
create_file_pointer_type!(ChunkPointer);

pub struct Chunk;

impl Chunk {
    fn load(address: ChunkPointer) -> Result<Chunk> {
        let address = address.0 << 4;
        Ok(Chunk)
    }
}

// TODO could async be used to make this thing more efficient at fetching nodes from the hard drive?
pub struct TerrainDiskStorage {
    index_file: File,
    chunk_file: File,
    loaded_nodes: HashMap<NodePointer, IndexNode>,
}

impl TerrainDiskStorage {
    pub fn initialize(mut index_file: File, mut chunk_file: File) -> Result<TerrainDiskStorage> {
        // TODO lock the file.

        let index_file_length = index_file.seek(SeekFrom::End(0))?;
        if index_file_length == 0 {
            // No need to seek back to the beginning, because this happens to also be it.

            // Create the index, with no root node. We'll add that in a moment.
            let mut index = TerrainDiskStorage { index_file, chunk_file, loaded_nodes: HashMap::new() };
            let root = index.new_node()?;
            debug_assert!(root.0 == 0);

            Ok(index)
        } else {
            Err(anyhow!("Expected newly created index file to be empty."))
        }
    }

    // TODO have a way to create the struct from an already existing pair of files.

    pub fn get_chunks_in_range(&mut self, high: (i16, i16, i16), low: (i16, i16, i16)) -> Result<ChunkIterator> {
        unimplemented!()
    }

    pub fn get_chunk(&mut self, x: i16, y: i16, z: i16) -> Result<Chunk> {
        let key = Self::create_chunk_key(x, y, z);
        let chunk_address = self.get_chunk_address(key).context("Error while indexing chunk.")?;

        Chunk::load(chunk_address).context("Error while loading chunk.")
    }

    fn get_chunk_address(&mut self, key: ChunkKey) -> Result<ChunkPointer> {
        let key_bytes = key.to_le_bytes();
        let layer1_key = u16::from_le_bytes(key_bytes[4..6].try_into().expect("Didn't get enough bytes for a key."));
        let layer2_key = u16::from_le_bytes(key_bytes[2..4].try_into().expect("Didn't get enough bytes for a key."));
        let layer3_key = u16::from_le_bytes(key_bytes[0..2].try_into().expect("Didn't get enough bytes for a key."));

        // Just a constant for code clarity.
        let root_node_pointer = NodePointer(0);
        // We start with the root and look for our first layer node.
        let layer2_node_address = self.get_node(root_node_pointer, |root| Ok(root.get_pointer(layer1_key)))?;

        let layer2_node_address = if let Some(address) = layer2_node_address {
            // The node already exists! We'll just use this address then.
            address
        } else {
            // The node does not exist. We must create it.
            let address = self.new_node()?;

            // Make sure to add that to the root node;
            self.get_node(root_node_pointer, |root| {
                root.set_pointer(layer1_key, address);
                Ok(())
            })?;

            address
        };

        let layer3_node_address = self.get_node(layer2_node_address, |node| {
            // We start with the root and look for our first layer node.
            Ok(node.get_pointer(layer2_key))
        })?;

        let layer3_node_address = if let Some(address) = layer3_node_address {
            // The node already exists! We'll just use this address then.
            address
        } else {
            // The node does not exist. We must create it.
            let address = self.new_node()?;

            // Make sure to add that to the root node;
            self.get_node(layer2_node_address, |node| {
                node.set_pointer(layer2_key, address);
                Ok(())
            })?;

            address
        };

        let chunk_address = self.get_node(layer3_node_address, |node| {
            // We start with the root and look for our first layer node.
            Ok(node.get_pointer(layer3_key))
        })?;

        let chunk_address = if let Some(address) = chunk_address {
            // The chunk already exists! We'll just use this address then.
            ChunkPointer(address.0)
        } else {
            // The chunk does not exist. We must create it.
            let address = self.new_chunk()?;

            self.get_node(layer2_node_address, |node| {
                // We start with the root and look for our first layer node.
                Ok(node.set_pointer(layer3_key, NodePointer(address.0)))
            })?;

            address
        };

        Ok(chunk_address)
    }

    fn new_chunk(&mut self) -> Result<ChunkPointer> {
        // Jump to the end.
        let pointer = self.chunk_file.seek(SeekFrom::End(0))?;

        // Now make the file longer to squeeze our node in.
        self.chunk_file.set_len(pointer + CHUNK_LENGTH)?;
        let pointer = ChunkPointer(pointer >> 4);

        Ok(pointer)
    }

    fn new_node(&mut self) -> Result<NodePointer> {
        // Jump to the end.
        let pointer = self.index_file.seek(SeekFrom::End(0))?;

        // Now make the file longer to squeeze our node in.
        self.index_file.set_len(pointer + CHUNK_LENGTH)?;
        let pointer = NodePointer(pointer);

        // We're probably about to need it, so go ahead and cache it.
        let node = IndexNode::load(&self.index_file, pointer)?;
        self.loaded_nodes.insert(pointer, node);

        Ok(pointer)
    }

    fn get_node<F: FnOnce(&mut IndexNode) -> Result<R>, R>(&mut self, pointer: NodePointer, function: F) -> Result<R> {
        let node = self.loaded_nodes.get_mut(&pointer);

        let node = if let Some(node) = node {
            // The node is loaded. Cool.
            node
        } else {
            // We need to load the node.
            // TODO somehow we need to know which node to unload when we have too many loaded.

            let node = IndexNode::load(&self.index_file, pointer)?;
            self.loaded_nodes.insert(pointer, node);

            self.loaded_nodes.get_mut(&pointer).expect("Node that was just inserted somehow wasn't found.")
        };

        // We can't safely return the node reference, so instead we call a provided function that will safely limit the lifetime of this reference.
        function(node)
    }

    fn create_chunk_key(x: i16, y: i16, z: i16) -> ChunkKey {
        // We group bits of the three axis together so that the more significant bits are on the left and the less significant are on the
        // right. This improves our chances of physically close chunks are close in the binary tree, improving our iteration speed when
        // requesting a range.
        fn spread_bits(input: i16) -> u64 {
            let mut input = input as u64 & 0x000000000000FFFF;
            let magic_numbers = [
                (32, 0x00FF00000000FFFF),
                (16, 0x00FF0000FF0000FF),
                (8, 0xF00F00F00F00F00F),
                (4, 0x30C30C30C30C30C3),
                (2, 0x9249249249249249),
            ];

            // TODO should be loop unrolling on its own but I should check this.
            for (shift, mask) in &magic_numbers {
                input = (input | (input << shift)) & mask;
            }

            input
        }

        let x = spread_bits(x);
        let y = spread_bits(y);
        let z = spread_bits(z);

        // Return all of these spaced out versions of the keys ored together.
        ChunkKey((x << 2) | (y << 1) | z)
    }

    pub fn flush(&mut self) -> Result<()> {
        for (_key, node) in &mut self.loaded_nodes {
            if node.is_modified() {
                node.flush()?;
            }
        }

        Ok(())
    }
}

struct IndexNode {
    modified: bool,
    memory: mapr::MmapMut,
}

impl IndexNode {
    fn load(file: &File, address: NodePointer) -> Result<IndexNode> {
        // Set the offset of our window into the file.
        let mut mmap_options = mapr::MmapOptions::new();
        mmap_options.offset(*address);

        // This is safe because we do not store any pointers for our application within that memory.
        // The worst someone could do by modifying that file during runtime is cause the wrong chunks to be fetched, or
        // to try and read outside of the file's range, which would result in a simple crash.
        Ok(IndexNode { modified: false, memory: unsafe { mmap_options.map_mut(file) }? })
    }

    fn get_pointer(&self, key: u16) -> Option<NodePointer> {
        let offset = (key * 8) as usize;
        let pointer =
            u64::from_le_bytes(self.memory[offset..offset + 8].try_into().expect("Not enough bytes to build file pointer."));

        if pointer != 0 {
            // We set the most significant bit to 1 so that even a pointer of zero appears as being set.
            Some(NodePointer(pointer & !0x8000_0000_0000_0000))
        } else {
            None
        }
    }

    fn set_pointer(&mut self, key: u16, address: NodePointer) {
        let offset = (key * 8) as usize;
        // The bitwise or is so that even a pointer of zero always is non-zero.
        let address = address.0 | 0x8000_0000_0000_0000;
        self.memory[offset..offset + 8].clone_from_slice(&address.to_le_bytes());
        self.modified = true;
    }

    fn is_modified(&self) -> bool {
        self.modified
    }

    fn flush(&mut self) -> Result<()> {
        self.memory.flush()?;
        self.modified = false;

        Ok(())
    }
}

pub struct ChunkIterator;

impl std::iter::Iterator for ChunkIterator {
    type Item = (u16, u16, u16, u64);

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

#[cfg(test)]
mod test_fileformate {

    use super::*;
    use tempfile::tempfile;

    #[test]
    fn insert_single_chunk_new_tree() {
        // let mut index = ChunkIndex::initialize(tempfile().unwrap()).unwrap();
        // index.create_chunk(0, 0, 0, ChunkPointer(0)).unwrap();
    }

    #[test]
    #[allow(overflowing_literals)] // Makes it so we can ignore the overflow when writing hexadecimal.
    fn chunk_keys() {
        // We test that every bit maps correctly.

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0000), ChunkKey(0x0000000000000000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x8000, 0x0000, 0x0000), ChunkKey(0x0000800000000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x8000, 0x0000), ChunkKey(0x0000400000000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x8000), ChunkKey(0x0000200000000000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x4000, 0x0000, 0x0000), ChunkKey(0x0000100000000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x4000, 0x0000), ChunkKey(0x0000080000000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x4000), ChunkKey(0x0000040000000000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x2000, 0x0000, 0x0000), ChunkKey(0x0000020000000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x2000, 0x0000), ChunkKey(0x0000010000000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x2000), ChunkKey(0x0000008000000000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x1000, 0x0000, 0x0000), ChunkKey(0x0000004000000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x1000, 0x0000), ChunkKey(0x0000002000000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x1000), ChunkKey(0x0000001000000000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0800, 0x0000, 0x0000), ChunkKey(0x0000000800000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0800, 0x0000), ChunkKey(0x0000000400000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0800), ChunkKey(0x0000000200000000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0400, 0x0000, 0x0000), ChunkKey(0x0000000100000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0400, 0x0000), ChunkKey(0x0000000080000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0400), ChunkKey(0x0000000040000000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0200, 0x0000, 0x0000), ChunkKey(0x0000000020000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0200, 0x0000), ChunkKey(0x0000000010000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0200), ChunkKey(0x0000000008000000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0100, 0x0000, 0x0000), ChunkKey(0x0000000004000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0100, 0x0000), ChunkKey(0x0000000002000000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0100), ChunkKey(0x0000000001000000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0080, 0x0000, 0x0000), ChunkKey(0x0000000000800000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0080, 0x0000), ChunkKey(0x0000000000400000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0080), ChunkKey(0x0000000000200000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0040, 0x0000, 0x0000), ChunkKey(0x0000000000100000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0040, 0x0000), ChunkKey(0x0000000000080000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0040), ChunkKey(0x0000000000040000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0020, 0x0000, 0x0000), ChunkKey(0x0000000000020000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0020, 0x0000), ChunkKey(0x0000000000010000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0020), ChunkKey(0x0000000000008000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0010, 0x0000, 0x0000), ChunkKey(0x0000000000004000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0010, 0x0000), ChunkKey(0x0000000000002000));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0010), ChunkKey(0x0000000000001000));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0008, 0x0000, 0x0000), ChunkKey(0x0000000000000800));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0008, 0x0000), ChunkKey(0x0000000000000400));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0008), ChunkKey(0x0000000000000200));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0004, 0x0000, 0x0000), ChunkKey(0x0000000000000100));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0004, 0x0000), ChunkKey(0x0000000000000080));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0004), ChunkKey(0x0000000000000040));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0002, 0x0000, 0x0000), ChunkKey(0x0000000000000020));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0002, 0x0000), ChunkKey(0x0000000000000010));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0002), ChunkKey(0x0000000000000008));

        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0001, 0x0000, 0x0000), ChunkKey(0x0000000000000004));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0001, 0x0000), ChunkKey(0x0000000000000002));
        assert_eq!(TerrainDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0001), ChunkKey(0x0000000000000001));
    }
}
