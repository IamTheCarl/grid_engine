// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Management of entity inventory and material/item transfers.

use core::hash::Hash;
use derive_error;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A unique ID to identify materials.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct MaterialID(u32);

impl Hash for MaterialID {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.0.hash(hasher)
    }
}

/// Information about a material.
#[derive(Serialize, Deserialize)]
pub struct MaterialInfo {
    name_tag: String,
    density: u64,
    material_id: MaterialID,
}

impl MaterialInfo {
    /// Get the name tag for this material.
    pub fn name_tag(&self) -> &str {
        &self.name_tag
    }

    /// Get the density of the material.
    pub fn density(&self) -> u64 {
        self.density
    }

    /// Get the unique key for identifying this material in this registry.
    pub fn id(&self) -> MaterialID {
        self.material_id
    }
}

impl<'a> Hash for MaterialInfo {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.material_id.hash(hasher)
    }
}

/// A collection of information about many materials.
pub struct MaterialRegistry {
    materials: Vec<MaterialInfo>,
    names_to_ids: HashMap<String, MaterialID>, // TODO might the slotmap be better for this?
}

impl MaterialRegistry {
    /// Create a new material registry.
    pub fn new() -> MaterialRegistry {
        MaterialRegistry { materials: Vec::new(), names_to_ids: HashMap::new() }
    }

    /// Register a new material with the registry.
    pub fn register_material(&mut self, name_tag: String, density: u64) {
        self.names_to_ids.insert(name_tag.clone(), MaterialID(self.materials.len() as u32));

        self.materials.push(MaterialInfo { name_tag, density, material_id: MaterialID(self.materials.len() as u32) });
    }

    /// Get the ID for a material.
    pub fn get_material_id(&self, name: &str) -> Option<MaterialID> {
        self.names_to_ids.get(name).copied()
    }

    /// Get information about a material by its ID.
    pub fn get_material_info(&self, material_id: MaterialID) -> Option<&MaterialInfo> {
        self.materials.get(material_id.0 as usize)
    }
}

/// A stack of material.
#[derive(Serialize, Deserialize)]
pub struct MaterialStack {
    material: MaterialID,
    quantity: u64,
}

impl MaterialStack {
    /// Create a new inventory stack of material.
    pub fn new(material: MaterialID, quantity: u64) -> MaterialStack {
        MaterialStack { material, quantity }
    }
}

impl Hash for MaterialStack {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.material.hash(hasher)
    }
}

/// A collection of many stacks of materials, plus items.
pub struct Inventory {
    material_stacks: HashSet<MaterialStack>,
    mass: u64,
    mass_limit: Option<u64>,
}

impl Inventory {
    /// Create an inventory with a limited capacity.
    pub fn limited(mass_limit: u64) -> Inventory {
        Inventory { material_stacks: HashSet::new(), mass: 0, mass_limit: Some(mass_limit) }
    }

    /// Create an inventory with no limit to its capacity.
    pub fn infinite() -> Inventory {
        Inventory { material_stacks: HashSet::new(), mass: 0, mass_limit: None }
    }

    /// Add or remove material in the inventory.
    pub fn add_material(&mut self, _material: MaterialID, _quantity: i64) {
        unimplemented!()
    }
}
