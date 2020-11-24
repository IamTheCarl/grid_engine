// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use rayon::{prelude::*, ThreadPool};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

mod storage;

// Names of files and folders in a world save.
const TERRAIN_FOLDER: &str = "terrain";

pub struct ChunkEntityKey;
pub struct GlobalEntityKey;

pub struct ChunkEntity;

pub struct GlobalEntity;

impl GlobalEntity {
    fn update(&mut self) {}
}

pub struct Chunk {
    storage: Option<Box<storage::ChunkData>>,
}

fn block_coordinate_to_chunk_coordinate(coordinate: (i64, i64, i64)) -> (i16, i16, i16) {
    (
        (coordinate.0 >> storage::BLOCK_ADDRESS_BITS) as i16,
        (coordinate.1 >> storage::BLOCK_ADDRESS_BITS) as i16,
        (coordinate.2 >> storage::BLOCK_ADDRESS_BITS) as i16,
    )
}

pub struct GridWorld {
    storage: storage::ChunkDiskStorage,
    terrain_chunks: HashMap<(i16, i16, i16), Chunk>,
    global_entities: HashMap<GlobalEntityKey, Box<GlobalEntity>>,
    chunk_entities: HashMap<ChunkEntityKey, Option<Box<ChunkEntity>>>,
}

impl GridWorld {
    pub fn new(folder: &Path) -> GridWorld {
        let storage = storage::ChunkDiskStorage::initialize(&folder.join(TERRAIN_FOLDER), 6);
        let terrain_chunks = HashMap::new();
        let global_entities = HashMap::new();
        let chunk_entities = HashMap::new();

        GridWorld { storage, terrain_chunks, global_entities, chunk_entities }
    }

    pub fn update(&mut self, thread_pool: &ThreadPool) {
        self.global_entities.iter_mut().for_each(|(_key, entity)| entity.update());
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
