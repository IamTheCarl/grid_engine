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

use slotmap::{new_key_type, SlotMap};

pub use proc_macros::Event;

pub mod inventory;
pub mod storage;
mod time;
pub use time::*;

// Names of files and folders in a world save.
const TERRAIN_FOLDER: &str = "terrain";

create_strong_type!(EventTypeID, u32);
new_key_type! { struct EntityID; }

/// Events must be serialized to be sent between entities. This container just keeps some essential data
/// in an unsterilized format for the engine to make use of.
#[derive(Eq, PartialEq)]
pub struct EventContainer {
    type_id: EventTypeID,
    source_entity_id: EntityID,
    target_component_name: String,
    serialized_data: Vec<u8>,
}

impl PartialOrd for EventContainer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.type_id.partial_cmp(&other.type_id)
    }
}

impl Ord for EventContainer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.type_id.cmp(&other.type_id)
    }
}

/// A message that one entity can send to another.
pub trait Event: Send + Serialize + Deserialize<'static> {
    /// The name that entities will use to identify this event's type.
    fn type_name() -> String;
}

trait EventSender {
    fn entities(&self) -> &SlotMap<EntityID, Mutex<Entity>>;

    fn entities_to_update_tx(&self) -> &mpsc::Sender<EntityID>;

    fn send_event_to_entity<EventData>(
        &self, target_entity_id: EntityID, source_entity_id: EntityID, type_id: EventTypeID, target_component_name: &str,
        event: &EventData,
    ) -> Result<()>
    where
        EventData: Event,
    {
        // Get the entity, push the event onto it.
        let entity = self.entities().get(target_entity_id).ok_or(anyhow!("Entity could not be found."))?;
        entity.lock().push_event(EventContainer {
            type_id,
            source_entity_id,
            target_component_name: String::from(target_component_name),
            serialized_data: serde_cbor::to_vec(&event)?,
        });

        // We got the entity earlier and didn't error out so we know this an id we can trust.
        // Mark it as needing to be updated.
        self.entities_to_update_tx().send(target_entity_id).context("Entity may have been dropped")?;

        Ok(())
    }
}

/// A part of an entity that processes events, adds behavior and storage.
trait Component: Send {
    fn process_event(&mut self, event: EventContainer, event_sender: &LocalEventSender) -> Result<()>;
}

/// An object that exists within the world.
pub struct Entity {
    events_to_process: Vec<EventContainer>,
    components: HashMap<String, Box<dyn Component>>,
}

impl Entity {
    fn push_event(&mut self, event: EventContainer) {
        self.events_to_process.push(event);
    }

    fn process_events(&mut self, event_sender: &LocalEventSender) {
        // Process events in order of priority. The event's type ID is what determines the priority.
        // Smaller numbers are higher priority.
        self.events_to_process.sort();

        // We consume the individual elements of the vector, leaving it empty after.
        for event in self.events_to_process.drain(..) {
            let component = self.components.get_mut(&event.target_component_name);

            // TODO make this error happen when the user first queues the event, rather than now.
            if let Some(component) = component {
                component.process_event(event, event_sender);
            } else {
                log::warn!(
                    "Tried to process event type {} on non existent component {}.",
                    event.type_id,
                    event.target_component_name
                );
            }
        }
    }
}

struct LocalEventSender<'a> {
    entities: &'a SlotMap<EntityID, Mutex<Entity>>,
    entities_to_update_tx: &'a mpsc::Sender<EntityID>,
}

impl<'a> EventSender for LocalEventSender<'a> {
    fn entities(&self) -> &SlotMap<EntityID, Mutex<Entity>> {
        self.entities
    }

    fn entities_to_update_tx(&self) -> &mpsc::Sender<EntityID> {
        self.entities_to_update_tx
    }
}

/// Used to track event type IDs and names.
pub struct EventTypeRegistry {
    event_type_ids: HashMap<String, u32>,
    event_type_names: Vec<String>,
}

impl EventTypeRegistry {
    /// Create a new event registry.
    pub fn new() -> EventTypeRegistry {
        EventTypeRegistry { event_type_ids: HashMap::new(), event_type_names: Vec::new() }
    }

    /// Register an event message so that it can be sent between entities.
    pub fn register_event_message<EventType>(&mut self, package_name: &str) -> Result<()>
    where
        EventType: Event,
    {
        self.register_event_message_raw(package_name, &EventType::type_name())
    }

    /// Register an event's type but you have to provide the event type name yourself.
    pub fn register_event_message_raw(&mut self, package_name: &str, event_name: &str) -> Result<()> {
        if !package_name.contains(':') && !event_name.contains(':') {
            let name = format!("{}:{}", package_name, event_name);
            self.event_type_ids.insert(name.clone(), self.event_type_ids.len() as u32);
            self.event_type_names.push(name);

            Ok(())
        } else {
            Err(anyhow!("An event's name cannot contain a '/'."))
        }
    }

    /// Get the id of an event type from its type name.
    pub fn get_event_type_id(&self, event_name: &str) -> Option<EventTypeID> {
        self.event_type_ids.get(event_name).map(|id| EventTypeID(*id))
    }

    /// Get the name of an event type from its type id.
    pub fn get_event_type_name(&self, event_id: EventTypeID) -> Option<&str> {
        self.event_type_names.get(event_id.0 as usize).map(|s| s.as_str())
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
    entities: SlotMap<EntityID, Mutex<Entity>>,
    entities_to_update_rx: mpsc::Receiver<EntityID>,
    entities_to_update_tx: mpsc::Sender<EntityID>,
    event_type_registry: EventTypeRegistry,
}

impl GridWorld {
    /// Create a new world with local storage.
    pub fn new(folder: &Path, event_type_registry: EventTypeRegistry) -> GridWorld {
        let storage = storage::ChunkDiskStorage::initialize(&folder.join(TERRAIN_FOLDER), 6);
        let terrain_chunks = HashMap::new();
        let time = WorldTime::from_ms(0);
        let entities = SlotMap::with_key();
        let next_entity_id = 0;
        let (entities_to_update_tx, entities_to_update_rx) = mpsc::channel();

        GridWorld { time, storage, terrain_chunks, entities, entities_to_update_rx, entities_to_update_tx, event_type_registry }
    }

    /// Get the event type registry for this world.
    pub fn get_event_type_registry(&self) -> &EventTypeRegistry {
        &self.event_type_registry
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
                        let entity = &entities[*entity_id];
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

    /// Create a new entity in the world.
    fn create_entity(&mut self, components: HashMap<String, Box<dyn Component>>) -> EntityID {
        self.entities.insert(Mutex::new(Entity { events_to_process: Vec::new(), components }))
    }
}

impl EventSender for GridWorld {
    fn entities(&self) -> &SlotMap<EntityID, Mutex<Entity>> {
        &self.entities
    }

    fn entities_to_update_tx(&self) -> &mpsc::Sender<EntityID> {
        &self.entities_to_update_tx
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use inventory::*;
    use tempfile::tempdir;

    #[test]
    fn register_events() {
        let mut registry = EventTypeRegistry::new();
        register_inventory_events(&mut registry).unwrap();

        assert_ne!(registry.get_event_type_id("core:MaterialAddEvent"), None);
        assert_ne!(registry.get_event_type_id("core:MaterialRejectEvent"), None);

        assert_ne!(registry.get_event_type_name(EventTypeID(0)), None);
        assert_ne!(registry.get_event_type_name(EventTypeID(1)), None);

        assert_ne!(registry.get_event_type_name(EventTypeID(0)), registry.get_event_type_name(EventTypeID(1)));
    }

    #[test]
    fn build_empty_entity() {
        let registry = EventTypeRegistry::new();
        let folder = tempdir().unwrap();

        let mut world = GridWorld::new(folder.path(), registry);
        let _id = world.create_entity(HashMap::new());
    }

    #[test]
    fn build_entity_with_inventory() {
        let mut event_registry = EventTypeRegistry::new();
        register_inventory_events(&mut event_registry).unwrap();

        let folder = tempdir().unwrap();

        let mut world = GridWorld::new(folder.path(), event_registry);
        let mut components: HashMap<String, Box<dyn Component>> = HashMap::new();

        let material_registry = MaterialRegistry::new();
        components.insert(String::from("inventory"), Box::new(Inventory::infinite(&material_registry)));

        let _id = world.create_entity(components);
    }
}
