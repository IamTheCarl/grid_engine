// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use anyhow::{anyhow, Context, Result};
use std::convert::TryInto;
use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;

// A chunk is 16x16x16 blocks in size, and a block consists of two bytes.
// That makes the chunk 8Kb in length.
const CHUNK_LENGTH: u64 = 16 * 16 * 16 * 2;

// A node contains 8 bits of addressable pointers, which point to more nodes, or chunks.
const NODE_LENGTH: u64 = 256 * 8;

create_file_pointer_type!(NodePointer);
create_file_pointer_type!(ChunkKey);
create_file_pointer_type!(ChunkPointer);

/// The raw data for a chunk's terrain.
pub struct TerrainChunkData {
    storage: [u16; 16 * 16 * 16],
    address: usize,
    x: i16,
    y: i16,
    z: i16,
}

impl<'a> TerrainChunkData {
    fn create(x: i16, y: i16, z: i16, address: ChunkPointer) -> Result<TerrainChunkData> {
        // Get the true address.
        let address = address.0 << 4;
        Ok(TerrainChunkData { storage: [0; 16 * 16 * 16], address: address as usize, x, y, z })
    }

    /// Gets the index of this chunk.
    pub fn get_index(&self) -> (i16, i16, i16) {
        (self.x, self.y, self.z)
    }

    /// Gets the address that this chunk is located at in the chunk file.
    pub fn get_address_in_file(&self) -> usize {
        self.address
    }

    /// Provides the block data for this chunk.
    pub fn get_data(&self) -> &[u16] {
        &self.storage
    }

    /// Provides the block data for this chunk.
    pub fn get_data_mut(&mut self) -> &mut [u16] {
        &mut self.storage
    }
}

/// A struct that will store and fetch chunks. It will create new chunks if the chunk does not exist in the file,
/// but it will not fill the chunk with content.
pub struct TerrainDiskStorage {
    index_file: File,
    chunk_file: File,
    index_memory: mapr::MmapMut,
    chunk_memory: mapr::MmapMut,
}

// Want to keep this thread safe.
static_assertions::assert_impl_all!(TerrainDiskStorage: Send, Sync);

impl TerrainDiskStorage {
    /// Provide a file handles for both the index file and the chunk file and this will be able to load and store
    /// terrain chunk data in them. Note that if the index file is uninitialized, this will go through the process of
    /// initializing them.
    pub fn initialize(mut index_file: File, mut chunk_file: File) -> Result<TerrainDiskStorage> {
        // TODO lock the files.

        // Get the length of the index file.
        let index_file_length = index_file.seek(SeekFrom::End(0))?;
        index_file.seek(SeekFrom::Start(0))?;
        if index_file_length == 0 {
            // We cannot have a non-zero length for a memory mapped file, so allocate memory for the root index node.
            index_file.set_len(NODE_LENGTH)?;
        }

        // Get the length of the chunk chunk file.
        let chunk_file_length = chunk_file.seek(SeekFrom::End(0))?;
        chunk_file.seek(SeekFrom::Start(0))?;
        if chunk_file_length == 0 {
            // We cannot have a non-zero length for a memory mapped file, so allocate memory for the root index node.
            chunk_file.set_len(CHUNK_LENGTH)?;
        }

        let index_memory = unsafe { mapr::MmapMut::map_mut(&index_file) }.context("Error while mapping index memory.")?;
        let chunk_memory = unsafe { mapr::MmapMut::map_mut(&chunk_file) }.context("Error while mapping chunk memory.")?;

        let index = TerrainDiskStorage { index_file, chunk_file, index_memory, chunk_memory };

        Ok(index)
    }

    /// Will get a single chunk's data at the specified chunk coordinates. Search time is O(1).
    /// If the chunk does not exist in the file, None will be returned.
    pub fn get_chunk(&self, x: i16, y: i16, z: i16) -> Result<Option<TerrainChunkData>> {
        let key = Self::create_chunk_key(x, y, z);
        let chunk_address = self.get_chunk_address(key).context("Error while indexing chunk.")?;

        if let Some(chunk_address) = chunk_address {
            let mut chunk = TerrainChunkData::create(x, y, z, chunk_address).context("Error while loading chunk.")?;

            self.load_chunk(&mut chunk, chunk_address)?;

            Ok(Some(chunk))
        } else {
            Ok(None)
        }
    }

    /// Will get a single chunk's data at the specified chunk coordinates. Search time is O(1).
    /// If the chunk does not exist in the file, it will be created. It also returns a boolean
    /// with it that is set to true if the chunk was just created.
    pub fn get_or_create_chunk(&mut self, x: i16, y: i16, z: i16) -> Result<(bool, TerrainChunkData)> {
        let key = Self::create_chunk_key(x, y, z);
        let (created, chunk_address) = self.get_or_create_chunk_address(key).context("Error while indexing chunk.")?;

        let mut chunk = TerrainChunkData::create(x, y, z, chunk_address).context("Error while loading chunk.")?;
        self.load_chunk(&mut chunk, chunk_address)?; // TODO that's a pointless operation. we should have this generate instead.

        Ok((created, chunk))
    }

    fn load_chunk(&self, chunk: &mut TerrainChunkData, chunk_address: ChunkPointer) -> Result<()> {
        // Load the data into it.
        let target = chunk.get_data_mut();
        let source = &self.chunk_memory[chunk_address.0 as usize..chunk_address.0 as usize + CHUNK_LENGTH as usize];

        // To do this efficiently, we have to do some odd iterating.
        let mut target_iterator = target.iter_mut();
        let mut source_iterator = source.iter();

        loop {
            let first = source_iterator.next();
            let second = source_iterator.next();
            let target = target_iterator.next();

            if let Some(first) = first {
                if let Some(second) = second {
                    if let Some(target) = target {
                        *target = u16::from_le_bytes([*first, *second]);
                        continue;
                    }
                }
            }

            // We only get here if one of the above if statements are false.
            // If they are all false, then that means we've successfully finished loading the chunk.
            if first.is_none() && second.is_none() && target.is_none() {
                break;
            } else {
                return Err(anyhow!("Unexpected end of chunk data in file."));
            }
        }

        Ok(())
    }

    /// Will flush all terrain index data to the hard drive.
    /// Will not flush chunk data to the hard drive.
    pub fn flush_index(&self) -> Result<()> {
        self.index_memory.flush()?;

        Ok(())
    }

    /// Returns the length of the chunk file in bytes.
    pub fn get_chunk_file_length(&mut self) -> Result<u64> {
        let length = self.chunk_file.seek(SeekFrom::End(0))?;
        self.chunk_file.seek(SeekFrom::Start(0))?;

        Ok(length)
    }

    /// Returns the length of the index file in bytes.
    pub fn get_index_file_length(&mut self) -> Result<u64> {
        let length = self.index_file.seek(SeekFrom::End(0))?;
        self.index_file.seek(SeekFrom::Start(0))?;

        Ok(length)
    }

    /// Save the bytes of a chunk to a file.
    pub fn save_chunk(&mut self, chunk: &TerrainChunkData) -> Result<()> {
        let chunk_address = chunk.get_address_in_file();

        let source = chunk.get_data();
        let target = &mut self.chunk_memory[chunk_address..chunk_address + CHUNK_LENGTH as usize];

        let mut target_iterator = target.iter_mut();
        for block in source {
            let first = target_iterator.next();
            let second = target_iterator.next();
            if let Some(first) = first {
                if let Some(second) = second {
                    let bytes = block.to_le_bytes();
                    *first = bytes[0];
                    *second = bytes[1];
                    continue;
                }
            }

            // If we get here, it means we ran out of space to store it.
            return Err(anyhow!("Chunk data is somehow longer than the storage space provided."));
        }

        Ok(())
    }

    /// Returns the address of a chunk. If the chunk does not exist in the index, none will be returned.
    fn get_chunk_address(&self, key: ChunkKey) -> Result<Option<ChunkPointer>> {
        let key_bytes = key.to_le_bytes();
        let keys = &key_bytes[3..7];
        let chunk_key = key_bytes[7];

        // We start with the root node.
        let mut node_address = NodePointer(0);

        for key in keys {
            // Try to get the node address.
            let next_node_address = self.get_node(node_address, |root| Ok(root.get_pointer(*key)))?;

            let next_node_address = if let Some(address) = next_node_address {
                // The node already exists! We'll just use this address then.
                address
            } else {
                return Ok(None);
            };

            // Step to the next node.
            node_address = next_node_address;
        }

        let chunk_address = self.get_node(node_address, |node| {
            // We start with the root and look for our first layer node.
            Ok(node.get_pointer(chunk_key))
        })?;

        if let Some(address) = chunk_address {
            // The chunk already exists! We'll just use this address then.
            Ok(Some(ChunkPointer(address.0)))
        } else {
            // The chunk does not exist.
            Ok(None)
        }
    }

    /// Returns the address of a chunk. If the chunk does not exist in the index, a chunk will be created and returned.
    /// With that chunk is returned a boolean. That boolean is true if the address was just created.
    fn get_or_create_chunk_address(&mut self, key: ChunkKey) -> Result<(bool, ChunkPointer)> {
        let key_bytes = key.to_le_bytes();
        let keys = &key_bytes[3..7];
        let chunk_key = key_bytes[7];
        let mut created = false;

        // We start with the root node.
        let mut node_address = NodePointer(0);

        for key in keys {
            // Try to get the node address.
            let next_node_address = self.get_node(node_address, |root| Ok(root.get_pointer(*key)))?;

            let next_node_address = if let Some(address) = next_node_address {
                // The node already exists! We'll just use this address then.
                address
            } else {
                // The node does not exist. We must create it.
                let address = self.new_node()?;

                // Make sure to add that to the root node;
                self.get_node_mut(node_address, |root| {
                    root.set_pointer(*key, address);
                    Ok(())
                })?;

                address
            };

            // Step to the next node.
            node_address = next_node_address;
        }

        let chunk_address = self.get_node(node_address, |node| {
            // We start with the root and look for our first layer node.
            Ok(node.get_pointer(chunk_key))
        })?;

        let chunk_address = if let Some(address) = chunk_address {
            // The chunk already exists! We'll just use this address then.
            ChunkPointer(address.0)
        } else {
            // The chunk does not exist. We must create it.
            let address = self.new_chunk()?;

            self.get_node_mut(node_address, |node| {
                created = true;

                // We start with the root and look for our first layer node.
                Ok(node.set_pointer(chunk_key, NodePointer(address.0)))
            })?;

            address
        };

        Ok((created, chunk_address))
    }

    fn new_chunk(&mut self) -> Result<ChunkPointer> {
        // Jump to the end.
        let mut pointer = self.chunk_file.seek(SeekFrom::End(0))?;
        if pointer == 1 {
            // This is actually the first chunk. We set a brand new file to a length of 1 bytes so we can map it into memory.
            pointer = 0;
        }

        debug_assert!(pointer & 0xFFF == 0);

        // Now make the file longer to squeeze our node in.
        self.chunk_file.set_len(pointer + CHUNK_LENGTH)?;
        let pointer = ChunkPointer(pointer >> 4);

        // TODO this may be very slow. Benchmarking is required, but if it is, then we need to resize this file with a smarter strategy.
        self.chunk_memory = unsafe { mapr::MmapMut::map_mut(&self.chunk_file) }.context("Error while mapping index memory.")?;

        Ok(pointer)
    }

    fn new_node(&mut self) -> Result<NodePointer> {
        // Jump to the end.
        let pointer = self.index_file.seek(SeekFrom::End(0))?;

        // Now make the file longer to squeeze our node in.
        self.index_file.set_len(pointer + NODE_LENGTH)?;
        let pointer = NodePointer(pointer);

        // This fails if we created a non-memory alined pointer.
        debug_assert!(pointer.0 & 0xFF == 0);

        // TODO this may be very slow. Benchmarking is required, but if it is, then we need to resize this file with a smarter strategy.
        self.index_memory = unsafe { mapr::MmapMut::map_mut(&self.index_file) }.context("Error while mapping index memory.")?;

        Ok(pointer)
    }

    fn get_node_mut<F: FnOnce(&mut IndexNode) -> Result<R>, R>(&mut self, pointer: NodePointer, function: F) -> Result<R> {
        let mut node = IndexNode::load(NodeMemoryReference::Mutable(&mut self.index_memory), *pointer)
            .context("Error while fetching node.")?;

        // We can't safely return the node reference, so instead we call a provided function that will safely limit the lifetime of this reference.
        function(&mut node)
    }

    fn get_node<F: FnOnce(&IndexNode) -> Result<R>, R>(&self, pointer: NodePointer, function: F) -> Result<R> {
        let node = IndexNode::load(NodeMemoryReference::Immutable(&self.index_memory), *pointer)
            .context("Error while fetching node.")?;

        // We can't safely return the node reference, so instead we call a provided function that will safely limit the lifetime of this reference.
        function(&node)
    }

    /// If you want to be able to fetch a chunk from the index, you first need a chunk key. This will generate it from a chunk index.
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
}

enum NodeMemoryReference<'a> {
    Mutable(&'a mut mapr::MmapMut),
    Immutable(&'a mapr::MmapMut),
}

struct IndexNode<'a> {
    memory: NodeMemoryReference<'a>,
    file_offset: usize,
}

impl<'a> IndexNode<'a> {
    fn load(memory: NodeMemoryReference, offset: u64) -> Result<IndexNode> {
        // Enforce that nodes are memory alined.
        if offset & 0xFF == 0 {
            Ok(IndexNode { memory, file_offset: offset as usize })
        } else {
            Err(anyhow!("Index Node is not memory alined: {:016x}", offset))
        }
    }

    fn get_pointer(&self, key: u8) -> Option<NodePointer> {
        match &self.memory {
            NodeMemoryReference::Mutable(memory) => self.get_pointer_internal(key, memory),
            NodeMemoryReference::Immutable(memory) => self.get_pointer_internal(key, memory),
        }
    }

    fn get_pointer_internal(&self, key: u8, memory: &mapr::MmapMut) -> Option<NodePointer> {
        let offset_key = self.file_offset + key as usize * 8;

        let pointer =
            u64::from_le_bytes(memory[offset_key..offset_key + 8].try_into().expect("Not enough bytes to build file pointer."));

        if pointer != 0 {
            // When storing this, we set the most significant bit to 1 so that even a pointer of zero appears as being set.
            // We don't want our caller to see that bit though, so clear it.
            Some(NodePointer(pointer & !0x8000_0000_0000_0000))
        } else {
            None
        }
    }

    fn set_pointer(&mut self, key: u8, address: NodePointer) {
        let offset_key = self.file_offset + key as usize * 8;
        // The bitwise or is so that even a pointer of zero is always non-zero.
        let address = address.0 | 0x8000_0000_0000_0000;

        match &mut self.memory {
            NodeMemoryReference::Mutable(memory) => memory[offset_key..offset_key + 8].clone_from_slice(&address.to_le_bytes()),
            NodeMemoryReference::Immutable(_memory) => panic!("Attempt to set pointer in an immutable node."),
        }
    }
}

#[cfg(test)]
#[allow(soft_unstable)] // Lets us use benchmarks in stable. Expect this to break some day.
mod test_fileformate {

    use super::*;
    use tempfile::tempfile;

    #[test]
    fn insert_single_chunk_new_file() {
        let mut index = TerrainDiskStorage::initialize(tempfile().unwrap(), tempfile().unwrap()).unwrap();
        index.get_or_create_chunk(0, 0, 0).unwrap();
        index.get_chunk(0, 0, 0).unwrap().unwrap();

        // Should be 5 nodes.
        assert_eq!(index.get_index_file_length().unwrap(), 10240);
    }

    #[test]
    fn get_chunk_that_does_not_exist() {
        let index = TerrainDiskStorage::initialize(tempfile().unwrap(), tempfile().unwrap()).unwrap();
        let chunk = index.get_chunk(0, 0, 0).unwrap();

        assert!(chunk.is_none());
    }

    #[test]
    fn save_chunk() {
        let mut index = TerrainDiskStorage::initialize(tempfile().unwrap(), tempfile().unwrap()).unwrap();
        let (_created, chunk) = index.get_or_create_chunk(0, 0, 0).unwrap();
        index.save_chunk(&chunk).unwrap();
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
