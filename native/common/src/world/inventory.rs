// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Management of entity inventory and material/item transfers.

use super::{Component, Event, EventContainer, LocalEventSender};
use crate::world::EventTypeRegistry;
use anyhow::Result;
use core::hash::Hash;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

/// A unique ID to identify materials.
pub struct MaterialID<'a>(u32, &'a PhantomData<u32>);

impl<'a> Hash for MaterialID<'a> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.0.hash(hasher)
    }
}

/// Information about a material.
struct Material<'a> {
    name_tag: String,
    density: f32,
    material_id: MaterialID<'a>,
}

impl<'a> Hash for Material<'a> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.material_id.hash(hasher)
    }
}

/// A collection of information about many materials.
pub struct MaterialRegistry<'a> {
    materials: Vec<Material<'a>>,
    names_to_ids: HashMap<String, MaterialID<'a>>,
}

/// A stack of material.
pub struct MaterialStack<'a> {
    material: &'a Material<'a>,
    quantity: u64,
}

impl<'a> Hash for MaterialStack<'a> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.material.hash(hasher)
    }
}

/// A collection of many stacks of materials, plus items.
pub struct Inventory<'a> {
    material_stacks: HashSet<MaterialStack<'a>>,
    mass: f32,
    mass_limit: Option<f32>,
}

impl<'a> Inventory<'a> {
    /// Create an inventory with a limited capacity.
    pub fn limited(_registry: &'a MaterialRegistry, mass_limit: f32) -> Inventory<'a> {
        Inventory { material_stacks: HashSet::new(), mass: 0.0, mass_limit: Some(mass_limit) }
    }

    /// Create an inventory with no limit to its capacity.
    pub fn infinite(_registry: &'a MaterialRegistry) -> Inventory<'a> {
        Inventory { material_stacks: HashSet::new(), mass: 0.0, mass_limit: None }
    }

    pub fn add_material(material_stack: MaterialStack) {}
}

const CORE_MODULE_NAME: &str = "core";

/// Register inventory events with the event registry.
pub fn register_inventory_events(registry: &mut EventTypeRegistry) -> Result<()> {
    registry.register_event_message::<MaterialAddEvent>(CORE_MODULE_NAME)?;
    registry.register_event_message::<MaterialRejectEvent>(CORE_MODULE_NAME)?;

    Ok(())
}

#[derive(Serialize, Deserialize, Event)]
struct MaterialAddEvent;

#[derive(Serialize, Deserialize, Event)]
struct MaterialRejectEvent;

impl<'a> Component for Inventory<'a> {
    fn process_event(&mut self, event: EventContainer, event_sender: &LocalEventSender) -> Result<()> {
        todo!()
    }
}
