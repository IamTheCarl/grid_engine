// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use antidote::Mutex;
use anyhow::{anyhow, Context, Result};
use core::cmp::{Eq, Ordering, PartialEq, PartialOrd};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub mod inventory;
pub mod storage;
mod time;
pub use time::*;

// Names of files and folders in a world save.
const TERRAIN_FOLDER: &str = "terrain";

create_strong_type!(EventTypeID);
create_strong_type!(EntityID);

/// A message that one entity can send to another.
#[derive(Eq, PartialEq)]
pub struct Event {
    type_id: EventTypeID,
    target_component: String,
    serialized_data: Vec<u8>,
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.type_id.partial_cmp(&other.type_id)
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.type_id.cmp(&other.type_id)
    }
}

trait EventSender {
    fn entities(&self) -> &HashMap<EntityID, Mutex<Entity>>;

    fn entities_to_update_tx(&self) -> &mpsc::Sender<EntityID>;

    fn send_event_to_entity(&self, entity_id: EntityID, event: Event) -> Result<()> {
        // Get the entity, push the event onto it.
        let entity = self.entities().get(&entity_id).ok_or(anyhow!("Entity could not be found."))?;
        entity.lock().push_event(event);

        // We got the entity earlier and didn't error out so we know this an id we can trust.
        // Mark it as needing to be updated.
        self.entities_to_update_tx().send(entity_id).context("Entity may have been dropped")?;

        Ok(())
    }
}

/// A part of an entity that processes events. (The plan here is still a bit fuzzy)
struct Component;

impl Component {
    fn process_event(&mut self, event: Event, event_sender: &LocalEventSender) {
        unimplemented!()
    }
}

/// An object that exists within the world.
pub struct Entity {
    events_to_process: Vec<Event>,
    components: HashMap<String, Component>,
}

impl Entity {
    fn create() -> Entity {
        unimplemented!()
    }

    fn push_event(&mut self, event: Event) {
        self.events_to_process.push(event);
    }

    fn process_events(&mut self, event_sender: &LocalEventSender) {
        // Process events in order of priority. The event's type ID is what determines the priority.
        // Smaller numbers are higher priority.
        self.events_to_process.sort();

        // We consume the individual elements of the vector, leaving it empty after.
        for event in self.events_to_process.drain(..) {
            let component = self.components.get_mut(&event.target_component);

            // TODO make this error happen when the user first queues the event, rather than now.
            if let Some(component) = component {
                component.process_event(event, event_sender);
            } else {
                log::warn!(
                    "Tried to process event type {} on non existent component {}.",
                    event.type_id,
                    event.target_component
                );
            }
        }
    }
}

struct LocalEventSender<'a> {
    entities: &'a HashMap<EntityID, Mutex<Entity>>,
    entities_to_update_tx: &'a mpsc::Sender<EntityID>,
}

impl<'a> EventSender for LocalEventSender<'a> {
    fn entities(&self) -> &HashMap<EntityID, Mutex<Entity>> {
        self.entities
    }

    fn entities_to_update_tx(&self) -> &mpsc::Sender<EntityID> {
        self.entities_to_update_tx
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
    time: WorldTime,
    storage: storage::ChunkDiskStorage,
    terrain_chunks: HashMap<(i16, i16, i16), Chunk>,
    entities: HashMap<EntityID, Mutex<Entity>>,
    entities_to_update_rx: mpsc::Receiver<EntityID>,
    entities_to_update_tx: mpsc::Sender<EntityID>,
}

impl GridWorld {
    pub fn new(folder: &Path, event_type_list: Vec<String>) -> GridWorld {
        let storage = storage::ChunkDiskStorage::initialize(&folder.join(TERRAIN_FOLDER), 6);
        let terrain_chunks = HashMap::new();
        let time = WorldTime::from_ms(0);
        let entities = HashMap::new();
        let (entities_to_update_tx, entities_to_update_rx) = mpsc::channel();

        GridWorld { time, storage, terrain_chunks, entities, entities_to_update_rx, entities_to_update_tx }
    }

    /// Update the entities of the world.
    /// Returns the count of events that were processed.
    pub fn update(&mut self) -> usize {
        // We are going to track the number of events that happened this frame.
        let mut num_events = 0;

        // We will loop until there are no more events left to process.
        // Processing of events can spawn more events, so this will likely take more than one iteration.
        // FIXME how do we prevent two entities from creating an endless cycle of events between each other?
        loop {
            // Remove all duplicates from the queue of entities we got. We use a HashSet to do that.
            let entities_to_update_set: HashSet<EntityID> = self.entities_to_update_rx.try_iter().collect();

            // Number of events to process.
            let events_processed = entities_to_update_set.len();
            // Were any events processed this iteration?
            if events_processed > 0 {
                // We have events to process!
                // Keep track of how many we processed.
                num_events += events_processed;

                // Have each entity process its events in parallel.
                entities_to_update_set.par_iter().for_each_with(
                    (&self.entities, self.entities_to_update_tx.clone()),
                    |(entities, entities_to_update_tx), entity_id| {
                        // We can't share entities_to_update_tx between threads safely, so we had to clone it.
                        let event_sender = LocalEventSender { entities, entities_to_update_tx };

                        // It shouldn't be possible for an entity to be deleted before its events are processed,
                        // so this should never panic.
                        let entity = &entities[entity_id];
                        entity.lock().process_events(&event_sender);
                    },
                );
            } else {
                // No events to process.
                // We can break out of the loop now.
                break;
            }
        }

        // Report how many events were processed
        num_events
    }
}

impl EventSender for GridWorld {
    fn entities(&self) -> &HashMap<EntityID, Mutex<Entity>> {
        &self.entities
    }

    fn entities_to_update_tx(&self) -> &mpsc::Sender<EntityID> {
        &self.entities_to_update_tx
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::tempdir;
}
