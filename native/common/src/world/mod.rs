// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use rayon::{prelude::*, ThreadPool};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::task::Poll;
use std::{collections::HashMap, path::Path};
mod storage;
mod time;
pub use time::*;

// Names of files and folders in a world save.
const TERRAIN_FOLDER: &str = "terrain";

create_strong_type!(ChunkEntityKey);
create_strong_type!(GlobalEntityKey);

pub struct ChunkEntity {
    associated_chunks: Vec<(i16, i16, i16)>,
}

pub struct GlobalEntity;

impl GlobalEntity {
    fn update(&mut self) {}
}

struct ChunkEntityFetchFuture<'a> {
    key: Option<&'a ChunkEntity>,
}

impl<'a> Future for ChunkEntityFetchFuture<'a> {
    type Output = &'a ChunkEntity;
    fn poll(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> Poll<<Self as std::future::Future>::Output> {
        if let Some(key) = self.key {
            Poll::Ready(key)
        } else {
            Poll::Pending
        }
    }
}

enum EntityCreationInstruction {
    Global { key: GlobalEntityKey },
    Chunk { key: ChunkEntityKey, chunks: Vec<(i16, i16, i16)> },
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
    chunk_entities: HashMap<ChunkEntityKey, Box<ChunkEntity>>,
    spawn_queue_rx: mpsc::Receiver<EntityCreationInstruction>,
    spawn_queue_tx: mpsc::Sender<EntityCreationInstruction>,
    time: WorldTime,
    global_entity_uid_count: std::sync::atomic::AtomicU64,
    chunk_entity_uid_count: std::sync::atomic::AtomicU64,
    world_id: [u8; 6],
}

impl GridWorld {
    pub fn new(folder: &Path) -> GridWorld {
        let storage = storage::ChunkDiskStorage::initialize(&folder.join(TERRAIN_FOLDER), 6);
        let terrain_chunks = HashMap::new();
        let global_entities = HashMap::new();
        let chunk_entities = HashMap::new();
        let (spawn_queue_tx, spawn_queue_rx) = mpsc::channel();
        let time = WorldTime::from_ms(0);
        let global_entity_uid_count = AtomicU64::new(0);
        let chunk_entity_uid_count = AtomicU64::new(0);
        let world_id = [0, 0, 0, 0, 0, 0];

        GridWorld {
            storage,
            terrain_chunks,
            global_entities,
            chunk_entities,
            spawn_queue_rx,
            spawn_queue_tx,
            time,
            chunk_entity_uid_count,
            global_entity_uid_count,
            world_id,
        }
    }

    fn new_global_entity(&self) -> GlobalEntityKey {
        let key = GlobalEntityKey(self.global_entity_uid_count.fetch_add(1, Ordering::Relaxed));

        // We don't actually create the entity now, but rather schedule to create it after all existing entities finish their update.
        self.spawn_queue_tx
            .send(EntityCreationInstruction::Global { key })
            .expect("Spawn queue receiver was dropped unexpectedly.");

        key
    }

    fn get_global_entity(&self, key: GlobalEntityKey) -> Option<&GlobalEntity> {
        if let Some(entity) = self.global_entities.get(&key) {
            Some(entity)
        } else {
            None
        }
    }

    fn new_chunk_entity(&self, chunks: Vec<(i16, i16, i16)>) -> ChunkEntityKey {
        let key = ChunkEntityKey(self.chunk_entity_uid_count.fetch_add(1, Ordering::Relaxed));

        // We don't actually create the entity now, but rather schedule to create it after all existing entities finish their update.
        self.spawn_queue_tx
            .send(EntityCreationInstruction::Chunk { key, chunks })
            .expect("Spawn queue receiver was dropped unexpectedly.");

        key
    }

    pub fn update(&mut self, thread_pool: &ThreadPool) {
        // Entities in the past frame demanded to spawn more entities.
        // Let's make that happen.
        for key in self.spawn_queue_rx.try_iter() {
            match key {
                EntityCreationInstruction::Global { key } => {
                    self.global_entities.insert(key, Box::new(GlobalEntity));
                }
                EntityCreationInstruction::Chunk { key, chunks } => {
                    self.chunk_entities.insert(key, Box::new(ChunkEntity { associated_chunks: chunks }));
                }
            }
        }

        // TODO in parallel.
        self.global_entities.iter_mut().for_each(|(_key, entity)| entity.update());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::tempdir;
}
