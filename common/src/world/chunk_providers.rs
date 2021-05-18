// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Chunk providers to fill your world with land and honey.

use super::{BlockID, BlockRegistry, Chunk, ChunkProvider};

/// Used by the terrain generator to indicate if the chunk has been fully generated or should be passed to the next generator
/// function to continue filling.
pub enum TerrainGeneratorSuccessType {
    /// Terrain generation is finished. Do not pass the chunk to the next generator.
    Finished,

    /// Terrain generation is unfinished. Please pass the chunk to the next generator.
    Continue,
}

/// A result indicating the success or failure of a generated chunk.
pub type TerrainGeneratorResult = anyhow::Result<TerrainGeneratorSuccessType>;

/// An object that provides the terrain for chunks.
pub trait TerrainGenerator<ChunkUserData: Default> {
    /// Load all the block IDs this generator needs to populate chunks.
    // TODO give this a way to fail if a block ID it needs is unavailable.
    fn initialize_block_ids(&mut self, registry: &mut BlockRegistry);

    /// Populates the provided chunk with terrain. Assumes the chunk is initially empty.
    fn populate_chunk(&self, chunk: &mut Chunk<ChunkUserData>) -> TerrainGeneratorResult;
}

/// Just a flat world of abstract blocks.
pub struct AbstractFlatWorld {
    abstract_block: Option<BlockID>,
}

impl AbstractFlatWorld {
    /// Create a new abstract flat world terrain provider.
    pub fn new() -> Box<AbstractFlatWorld> {
        Box::new(AbstractFlatWorld { abstract_block: None })
    }
}

impl<ChunkUserData: Default> TerrainGenerator<ChunkUserData> for AbstractFlatWorld {
    fn initialize_block_ids(&mut self, registry: &mut BlockRegistry) {
        // These should never fail since they're the only ones I'm adding, but if it does fail we just ignore that failure.
        registry.add_block(String::from("abstract_block"), String::from("Abstract Block")).ok();

        // Notice that if the block does not exist in the registry, this will just start setting all the blocks for this one to none.
        self.abstract_block = registry.get_block_id_from_name("abstract_block").cloned();
    }

    fn populate_chunk(&self, chunk: &mut Chunk<ChunkUserData>) -> TerrainGeneratorResult {
        // TODO provide a way to properly fail.
        // We just generate a flat world.
        let chunk_location = chunk.index();
        if chunk_location.y < 0 {
            chunk.iter_ideal_mut(Chunk::<ChunkUserData>::range_all_blocks()).for_each(|block| *block = self.abstract_block);
        }

        Ok(TerrainGeneratorSuccessType::Finished)
    }
}

/// A world that just exists in memory. It cannot be saved or backed up.
/// It's ideal for testing!
pub struct RAMWorld<ChunkUserData> {
    block_registry: BlockRegistry,
    generators: Vec<Box<dyn TerrainGenerator<ChunkUserData>>>,
}

impl<ChunkUserData: Default> RAMWorld<ChunkUserData> {
    /// Construct a new RAM world.
    pub fn new(block_registry: BlockRegistry) -> Box<RAMWorld<ChunkUserData>> {
        let generators = Vec::new();

        Box::new(RAMWorld { block_registry, generators })
    }

    // TODO this should definitely go into a factory.
    /// Add a terrain generator to provide terrain for this chunk. Terrain generators will be called in the order
    /// they have been added. If the generators returns "failed" a warning message will be logged and the next generator will be used.
    /// If the generator returns "continue" then the next generator will be called. If  the generator returns "finished" then the next
    /// terrain generator will not be called. In the case that there is no next terrain generator, then this function will return.
    pub fn add_generator(&mut self, mut generator: Box<dyn TerrainGenerator<ChunkUserData>>) {
        generator.initialize_block_ids(&mut self.block_registry);
        self.generators.push(generator);
    }
}

impl<ChunkUserData: Default> ChunkProvider<ChunkUserData> for RAMWorld<ChunkUserData> {
    fn provide_chunk(&self, chunk: &mut Chunk<ChunkUserData>) {
        for generator in self.generators.iter() {
            match generator.populate_chunk(chunk) {
                Ok(success_type) => match success_type {
                    TerrainGeneratorSuccessType::Continue => continue,
                    TerrainGeneratorSuccessType::Finished => break,
                },
                Err(error) => {
                    log::error!("Fatal error while populating chunk: {:?}", error);
                }
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
