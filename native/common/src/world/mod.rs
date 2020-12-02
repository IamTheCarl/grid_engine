// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use serde::{Deserialize, Serialize};
use std::any::Any;
use std::{
    collections::{HashMap, VecDeque},
    path::Path,
};

pub mod storage;
mod time;
pub use time::*;

// Names of files and folders in a world save.
const TERRAIN_FOLDER: &str = "terrain";

create_strong_type!(EventTypeID);

struct Event {
    // TODO I prefer not to do runtime reflection. See if you can  do away with that.
    payload: dyn Any,
}

create_strong_type!(ChunkEntityKey);
create_strong_type!(GlobalEntityKey);

pub struct ChunkEntity {
    associated_chunks: Vec<(i16, i16, i16)>,
}

pub struct GlobalEntity {
    event_queue: VecDeque<Box<Event>>,
}

impl GlobalEntity {
    fn create() -> GlobalEntity {
        unimplemented!()
    }

    fn push_event(&mut self, event: Box<Event>) {
        self.event_queue.push_back(event);
    }

    fn process_events(&mut self) {
        unimplemented!()
    }
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
    time: WorldTime,
    next_global_entity_uid: u64,
    next_chunk_entity_uid: u64,
    // event_name_to_id_map: HashMap<String, EventTypeID>,
    // event_id_to_name_map: Vec<String>,
}

impl GridWorld {
    pub fn new(folder: &Path, event_type_list: Vec<String>) -> GridWorld {
        let storage = storage::ChunkDiskStorage::initialize(&folder.join(TERRAIN_FOLDER), 6);
        let terrain_chunks = HashMap::new();
        let global_entities = HashMap::new();
        let chunk_entities = HashMap::new();
        let time = WorldTime::from_ms(0);
        let next_global_entity_uid = 0;
        let next_chunk_entity_uid = 0;

        GridWorld {
            storage,
            terrain_chunks,
            global_entities,
            chunk_entities,
            time,
            next_chunk_entity_uid,
            next_global_entity_uid,
        }
    }

    fn new_global_entity(&mut self) -> GlobalEntityKey {
        let key = GlobalEntityKey(self.next_global_entity_uid);
        self.next_global_entity_uid += 1;
        key
    }

    fn get_global_entity(&self, key: GlobalEntityKey) -> Option<&GlobalEntity> {
        if let Some(entity) = self.global_entities.get(&key) {
            Some(entity)
        } else {
            None
        }
    }

    fn new_chunk_entity(&mut self, chunks: Vec<(i16, i16, i16)>) -> ChunkEntityKey {
        let key = ChunkEntityKey(self.next_chunk_entity_uid);
        self.next_chunk_entity_uid += 1;
        key
    }

    pub fn update(&mut self) {}
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::tempdir;
}
