// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Chunk providers to fill your world with land and honey.

use super::{storage::CHUNK_DIAMETER, BlockID, BlockRegistry, Chunk, ChunkProvider, LocalBlockCoordinate, LocalBlockRange};

/// A world that just exists in memory. It cannot be saved or backed up.
/// It's ideal for testing!
pub struct RAMWorld {
    block_registry: BlockRegistry,
    abstract_block: BlockID,
}

impl RAMWorld {
    /// Construct a new RAM world.
    pub fn new() -> RAMWorld {
        let mut block_registry = BlockRegistry::new();

        // These should never fail since they're the only ones I'm adding.
        block_registry.add_block(String::from("abstract_block"), String::from("Abstract Block")).unwrap();
        let abstract_block = block_registry.get_block_data_from_name("abstract_block").unwrap().id;

        RAMWorld { block_registry, abstract_block }
    }
}

impl ChunkProvider for RAMWorld {
    fn provide_chunk(&self, chunk: &mut Chunk) {
        // We just generate a flat world.
        let chunk_location = chunk.index();

        if chunk_location.y == 0 {
            for block in chunk.iter_ideal_mut(LocalBlockRange::from_end_points(
                LocalBlockCoordinate::new(0, 0, 0),
                LocalBlockCoordinate::new(CHUNK_DIAMETER as u8, 0, CHUNK_DIAMETER as u8),
            )) {
                *block = Some(self.abstract_block);
            }
        }
    }
    fn block_registry(&self) -> &BlockRegistry {
        &self.block_registry
    }
    fn block_registry_mut(&mut self) -> &mut BlockRegistry {
        &mut self.block_registry
    }
}
