// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Modules are how user made content are loaded into the engine.
//! 
//! Modules are zip archives (typically compressed) containing web assembly code to add content to the game, along
//! with additional assets they may depend on.
//! 
//! Currently they are only able to be manually imported.
//! There is no automated download of modules.
//! Dependency resolution is unfinished.

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
/// Metadata of a module package.
pub struct PackageMetadata {
    /// A revision number to check compatibility.
    pub revision: u16,
    /// The name of the package.
    pub name: String,
}

// pub struct Module {
//     name: String,
//     wasm_binaries: Vec<WasmBinary>
// }
