// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

// Expected size of a block on the hard drive.
const BLOCK_SIZE: u64 = 4096;

// There is a header at the beginning of the tree file. This is how big it is.
const HEADER_SIZE: u64 = BLOCK_SIZE;

// Actual size of a node in the index file.
const NODE_LENGTH: u64 = BLOCK_SIZE;

const NODE_MIN: usize = 0;
const NODE_MAX: usize = 1;
// An unused reserved space.
const NODE_PARENT: usize = 8;
const NODE_GREATER: usize = 16;
const NODE_LESSER: usize = 24;
const NODE_POINTERS_START: usize = 32;

const NODE_POINTERS_MAX_INDEX: u8 = ((NODE_LENGTH - NODE_POINTERS_START as u64) / 16) as u8;
const NODE_POINTERS_CENTER_INDEX: u8 = NODE_POINTERS_MAX_INDEX / 2;

create_file_pointer_type!(NodePointer);
create_file_pointer_type!(ChunkPointer);
create_file_pointer_type!(ChunkKey);

// TODO could async be used to make this thing more efficient at fetching nodes from the hard drive?
pub struct ChunkIndex {
    file: File,
    root: NodePointer,
    loaded_nodes: HashMap<NodePointer, ChunkNode>,
}

impl ChunkIndex {
    pub fn initialize(mut file: File) -> Result<ChunkIndex> {
        // TODO lock the file.

        let file_length = file.seek(SeekFrom::End(0))?;
        if file_length == 0 {
            // No need to seek back to the beginning, because this happens to also be it.

            // Take a block of the file for our header. This will contain things like a version hash and
            // a pointer to the root node.
            file.set_len(HEADER_SIZE)?;

            // Create the index, with no root node. We'll add that in a moment.
            let mut index = ChunkIndex { file, root: NodePointer(0), loaded_nodes: HashMap::new() };
            let root = index.new_node(NodePointer(0))?;
            index.root = root;

            Ok(index)
        } else {
            Err(anyhow!("Expected newly created index file to be empty."))
        }
    }

    pub fn get_chunk_pointer(&mut self, x: i16, y: i16, z: i16) -> Result<u64> {
        unimplemented!()
    }

    pub fn get_chunks_in_range(&mut self, high: (i16, i16, i16), low: (i16, i16, i16)) -> Result<ChunkIterator> {
        unimplemented!()
    }

    pub fn create_chunk(&mut self, x: i16, y: i16, z: i16, pointer: ChunkPointer) -> Result<()> {
        let key = Self::create_chunk_key(x, y, z);
        let mut node = self.root;
        let mut pointer = pointer;

        loop {
            let result = self.get_node(node, |node| Ok(node.add_chunk(key, pointer)))?;
            match result {
                AddChunkResult::Ok => {
                    // Alright, we added it. Save and break out of this loop.
                    self.get_node(node, |node| Ok(node.save()))??;
                    break;
                }
                AddChunkResult::Lesser(address, to_move) | AddChunkResult::Greater(address, to_move) => {
                    // There wasn't space for this chunk in there, so it's ether going into a child node, or this one got bumped out and needs
                    // to be moved into a child node.
                    // node = address;
                    // pointer = to_move;
                    Ok(())
                }
                // AddChunkResult::CreateLesser => {
                //     let parent = node;
                //     node = self.new_node(parent)?;
                //     self.get_node(parent, |parent| {
                //         parent.set_lesser(node);
                //         parent.save()?;

                //         Ok(())
                //     })?;
                //     Ok(())
                // }
                // AddChunkResult::CreateGreater => {
                //     let parent = node;
                //     node = self.new_node(parent)?;
                //     self.get_node(parent, |parent| {
                //         parent.set_greater(node);
                //         parent.save()?;

                //         Ok(())
                //     })?;
                //     Ok(())
                // } // TODO you gotta balance this thing somehow.
                AddChunkResult::AlreadyExists => Err(anyhow!("Chunk already exists in index.")),
            }?;
        }

        Ok(())
    }

    fn new_node(&mut self, parent: NodePointer) -> Result<NodePointer> {
        // Jump to the end and get our length.
        let pointer = self.file.seek(SeekFrom::End(0))?;

        // Now make the file longer to squeeze our node in.
        self.file.set_len(pointer + NODE_LENGTH)?;

        // TODO we should use the actual struct to do this.

        let parent = parent.to_le_bytes();

        self.file.write(&[
            NODE_POINTERS_CENTER_INDEX, // Min
            NODE_POINTERS_CENTER_INDEX, // Max
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00, // Reserved
            parent[0],
            parent[1],
            parent[2],
            parent[3],
            parent[4],
            parent[5],
            parent[6],
            parent[7], // Parent pointer.

                       // We leave the two child node pointers null (zeros).
        ])?;

        Ok(NodePointer(pointer))
    }

    fn get_node<F: FnOnce(&mut ChunkNode) -> Result<R>, R>(&mut self, pointer: NodePointer, function: F) -> Result<R> {
        let node = self.loaded_nodes.get_mut(&pointer);

        let node = if let Some(node) = node {
            // The node is loaded. Cool.
            node
        } else {
            // We need to load the node.
            // TODO somehow we need to know which node to unload when we have too many loaded.

            let node = ChunkNode::load(&self.file, pointer)?;
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
}

struct ChunkNode {
    memory: mapr::MmapMut,
}

enum AddChunkResult {
    Ok,
    Lesser(ChunkKey, ChunkPointer),
    Greater(ChunkKey, ChunkPointer),
    AlreadyExists,
}

impl ChunkNode {
    fn load(file: &File, address: NodePointer) -> Result<ChunkNode> {
        // Set the offset of our window into the file.
        let mut mmap_options = mapr::MmapOptions::new();
        mmap_options.offset(*address);

        // This is safe because we do not store any pointers for our application within that memory.
        // The worst someone could do by modifying that file during runtime is cause the wrong chunks to be fetched, or
        // to try and read outside of the file's range, which would result in a simple crash.
        Ok(ChunkNode { memory: unsafe { mmap_options.map_mut(file) }? })
    }

    fn set_lesser(&mut self, address: u64) {
        let address = address.to_le_bytes();
        self.memory[NODE_LESSER..NODE_LESSER + 7].clone_from_slice(&address);
    }

    fn set_greater(&mut self, address: u64) {
        let address = address.to_le_bytes();
        self.memory[NODE_GREATER..NODE_GREATER + 7].clone_from_slice(&address);
    }

    fn index_min(&self) -> u8 {
        self.memory[NODE_MIN]
    }

    fn index_max(&self) -> u8 {
        self.memory[NODE_MAX]
    }

    fn index_min_mut(&mut self) -> &mut u8 {
        &mut self.memory[NODE_MIN]
    }

    fn index_max_mut(&mut self) -> &mut u8 {
        &mut self.memory[NODE_MAX]
    }

    fn is_space_for_lesser(&self) -> bool {
        self.index_min() > 0
    }

    fn is_space_for_greater(&self) -> bool {
        self.index_max() > NODE_POINTERS_MAX_INDEX
    }

    fn is_empty(&self) -> bool {
        self.index_max() == self.index_min()
    }

    fn lesser(&self) -> Option<u64> {
        let value: [u8; 8] =
            self.memory[NODE_LESSER..NODE_LESSER + 7].try_into().expect("Didn't get enough bytes for pointer.");
        let value = u64::from_le_bytes(value);

        if value != 0 {
            Some(value)
        } else {
            None
        }
    }

    fn greater(&self) -> Option<u64> {
        let value: [u8; 8] =
            self.memory[NODE_GREATER..NODE_GREATER + 7].try_into().expect("Didn't get enough bytes for pointer.");
        let value = u64::from_le_bytes(value);

        if value != 0 {
            Some(value)
        } else {
            None
        }
    }

    fn get_pointer_set(&self, index: u8) -> Option<(ChunkKey, ChunkPointer)> {
        if index >= self.index_min() && index < self.index_max() {
            // It's in range. Get it.

            let index = NODE_POINTERS_START + index as usize * 16;
            let key = &self.memory[index..index + 8];
            let address = &self.memory[index + 8..index + 16];
            let key = u64::from_le_bytes(key.try_into().expect("Didn't get enough bytes for a key."));
            let address = u64::from_le_bytes(address.try_into().expect("Didn't get enough bytes for a chunk address."));
            Some((ChunkKey(key), ChunkPointer(address)))
        } else {
            // Index is out of range or we are empty. Return none.
            None
        }
    }

    fn set_pointer_set(&mut self, index: u8, key: ChunkKey, address: ChunkPointer) {
        let key = key.to_le_bytes();
        let address = address.to_le_bytes();

        let index = NODE_POINTERS_START + index as usize * 16;
        self.memory[index..index + 8].clone_from_slice(&address);
        self.memory[index + 8..index + 16].clone_from_slice(&address);
    }

    fn insert(&mut self, index: u8, key: ChunkKey, address: ChunkPointer) -> std::result::Result<(), (ChunkKey, ChunkPointer)> {
        unimplemented!()
    }

    fn add_chunk(&mut self, key: ChunkKey, address: ChunkPointer) -> AddChunkResult {
        if !self.is_empty() {
            let mut near = self.index_min();
            let mut far = self.index_max();

            loop {
                let middle = near + (near - far) / 2;

                let pointer_set = self.get_pointer_set(middle);

                if let Some((check_key, _address)) = pointer_set {
                    if near == far {
                        // Oh, so this is the insertion point.

                        // Should be impossible at this point.
                        assert_ne!(key, check_key);

                        let is_greater = key > check_key;

                        // In the case that we are greater, we go just after the point.
                        let insertion_point = if is_greater { middle + 1 } else { middle };

                        // TODO this may not fit and need to be put into a child node.
                        let overflow = self.insert(insertion_point, key, address);
                        if let Err((overflow_key, overflow_pointer)) = overflow {
                            // We didn't have enough space to fit it. Now this pointer needs to be packed into a child node.
                            if is_greater {
                                return AddChunkResult::Greater(overflow_key, overflow_pointer);
                            } else {
                                return AddChunkResult::Lesser(overflow_key, overflow_pointer);
                            }
                        }
                    }

                    // Alright, so this is in range. Is it bigger than us, or smaller than us?
                    if key > check_key {
                        // We are bigger. We need to search in the greater half.
                        near = middle;
                    } else if key < check_key {
                        // We are smaller. We need to search in the lower half.
                        far = middle;
                    } else {
                        // We are equal, we already have this chunk.
                        return AddChunkResult::AlreadyExists;
                    }
                } else {
                    // Okay, so this was out of the node's range. This means we'll want to append here (assuming there's space).
                }
            }
        } else {
            // We add to ourself no matter what here.
            let index_min = self.index_min();
            self.set_pointer_set(index_min, key, address);
            *self.index_min_mut() -= 1; // We add it in the down direction.

            // You got the good ending.
            AddChunkResult::Ok
        }
    }

    fn save(&mut self) -> Result<()> {
        self.memory.flush()?;

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
        let mut index = ChunkIndex::initialize(tempfile().unwrap()).unwrap();
        index.create_chunk(0, 0, 0, ChunkPointer(0)).unwrap();
    }

    #[test]
    fn insert_single_chunk_directly_into_node() {
        let mut index = ChunkIndex::initialize(tempfile().unwrap()).unwrap();
        let node = index.root;
        let key = ChunkIndex::create_chunk_key(0, 0, 0);
        let pointer = ChunkPointer(0);

        let result = index.get_node(node, |node| Ok(node.add_chunk(key, pointer))).unwrap();
        match result {
            AddChunkResult::Ok => {}
            _ => panic!("Wrong result."),
        }
    }

    #[test]
    #[allow(overflowing_literals)] // Makes it so we can ignore the overflow when writing hexadecimal.
    fn chunk_keys() {
        // We test that every bit maps correctly.

        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0000), ChunkKey(0x0000000000000000));

        assert_eq!(ChunkIndex::create_chunk_key(0x8000, 0x0000, 0x0000), ChunkKey(0x0000800000000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x8000, 0x0000), ChunkKey(0x0000400000000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x8000), ChunkKey(0x0000200000000000));

        assert_eq!(ChunkIndex::create_chunk_key(0x4000, 0x0000, 0x0000), ChunkKey(0x0000100000000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x4000, 0x0000), ChunkKey(0x0000080000000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x4000), ChunkKey(0x0000040000000000));

        assert_eq!(ChunkIndex::create_chunk_key(0x2000, 0x0000, 0x0000), ChunkKey(0x0000020000000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x2000, 0x0000), ChunkKey(0x0000010000000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x2000), ChunkKey(0x0000008000000000));

        assert_eq!(ChunkIndex::create_chunk_key(0x1000, 0x0000, 0x0000), ChunkKey(0x0000004000000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x1000, 0x0000), ChunkKey(0x0000002000000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x1000), ChunkKey(0x0000001000000000));

        assert_eq!(ChunkIndex::create_chunk_key(0x0800, 0x0000, 0x0000), ChunkKey(0x0000000800000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0800, 0x0000), ChunkKey(0x0000000400000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0800), ChunkKey(0x0000000200000000));

        assert_eq!(ChunkIndex::create_chunk_key(0x0400, 0x0000, 0x0000), ChunkKey(0x0000000100000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0400, 0x0000), ChunkKey(0x0000000080000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0400), ChunkKey(0x0000000040000000));

        assert_eq!(ChunkIndex::create_chunk_key(0x0200, 0x0000, 0x0000), ChunkKey(0x0000000020000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0200, 0x0000), ChunkKey(0x0000000010000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0200), ChunkKey(0x0000000008000000));

        assert_eq!(ChunkIndex::create_chunk_key(0x0100, 0x0000, 0x0000), ChunkKey(0x0000000004000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0100, 0x0000), ChunkKey(0x0000000002000000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0100), ChunkKey(0x0000000001000000));

        assert_eq!(ChunkIndex::create_chunk_key(0x0080, 0x0000, 0x0000), ChunkKey(0x0000000000800000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0080, 0x0000), ChunkKey(0x0000000000400000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0080), ChunkKey(0x0000000000200000));

        assert_eq!(ChunkIndex::create_chunk_key(0x0040, 0x0000, 0x0000), ChunkKey(0x0000000000100000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0040, 0x0000), ChunkKey(0x0000000000080000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0040), ChunkKey(0x0000000000040000));

        assert_eq!(ChunkIndex::create_chunk_key(0x0020, 0x0000, 0x0000), ChunkKey(0x0000000000020000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0020, 0x0000), ChunkKey(0x0000000000010000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0020), ChunkKey(0x0000000000008000));

        assert_eq!(ChunkIndex::create_chunk_key(0x0010, 0x0000, 0x0000), ChunkKey(0x0000000000004000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0010, 0x0000), ChunkKey(0x0000000000002000));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0010), ChunkKey(0x0000000000001000));

        assert_eq!(ChunkIndex::create_chunk_key(0x0008, 0x0000, 0x0000), ChunkKey(0x0000000000000800));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0008, 0x0000), ChunkKey(0x0000000000000400));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0008), ChunkKey(0x0000000000000200));

        assert_eq!(ChunkIndex::create_chunk_key(0x0004, 0x0000, 0x0000), ChunkKey(0x0000000000000100));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0004, 0x0000), ChunkKey(0x0000000000000080));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0004), ChunkKey(0x0000000000000040));

        assert_eq!(ChunkIndex::create_chunk_key(0x0002, 0x0000, 0x0000), ChunkKey(0x0000000000000020));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0002, 0x0000), ChunkKey(0x0000000000000010));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0002), ChunkKey(0x0000000000000008));

        assert_eq!(ChunkIndex::create_chunk_key(0x0001, 0x0000, 0x0000), ChunkKey(0x0000000000000004));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0001, 0x0000), ChunkKey(0x0000000000000002));
        assert_eq!(ChunkIndex::create_chunk_key(0x0000, 0x0000, 0x0001), ChunkKey(0x0000000000000001));
    }
}
