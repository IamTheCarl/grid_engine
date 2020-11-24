// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Modules are how user made content are loaded into the engine.
//!
//! Modules are zip archives (typically compressed) containing web assembly code
//! to add content to the game, along with additional assets they may depend on.
//!
//! Currently they are only able to be manually imported.
//! There is no automated download of modules.
//! Dependency resolution is unfinished.

use anyhow::{anyhow, Result};
use io::{Read, Seek};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::HashMap,
    io,
    path::{Path, PathBuf},
};
use zip::read::{ZipArchive, ZipFile};

#[derive(Serialize, Deserialize)]
/// Metadata of a module package.
pub struct PackageMetadata {
    /// A revision number to check compatibility.
    pub revision: u16,
    /// The name of the package.
    pub name: String,
}

/// An index of a module package.
/// Does not actually load the whole package into memory. It just loads an index
/// and provides an easy interface to load the data from the package.
pub struct PackageFile<R: Read + Seek> {
    archive: ZipArchive<R>,
    metadata: PackageMetadata,
    wasm: HashMap<PathBuf, usize>,
}

impl<R: Read + Seek> PackageFile<R> {
    /// Build a new package file from a seek-able input source.
    pub fn load(source: R) -> Result<PackageFile<R>> {
        let mut archive = ZipArchive::new(source)?;
        let mut metadata: Option<PackageMetadata> = None;
        let mut nonstandard_paths = Vec::new();
        let mut wasm = HashMap::new();

        // Build a usable index of the archive.
        for index in 0..archive.len() {
            let file = archive.by_index(index)?;
            let file_name = file.name();

            match file_name {
                "META" => {
                    metadata = Some(bincode::deserialize_from(file)?);
                }
                _ => {
                    let file_path = PathBuf::from(file.name());
                    if file_path.starts_with("wasm") {
                        let path =
                            PathBuf::from(file_path.strip_prefix("wasm").expect("A file under binary is not under binary."));
                        log::debug!("Registered wasm resource: {:?}", path);
                        wasm.insert(path, index);
                    } else {
                        // We log all of them together when we're done.
                        // We do this so
                        nonstandard_paths.push(format!("{}\n", file_name));
                    }
                }
            }
        }

        // Check to make sure everything we need is there and at valid locations.
        if let Some(metadata) = metadata {
            if nonstandard_paths.is_empty() {
                Ok(PackageFile { archive, metadata, wasm })
            } else {
                let mut files = String::default();

                for file in nonstandard_paths {
                    files += &file;
                }

                Err(anyhow!("There are non-standard files in the package {}:\n{}", metadata.name, files))
            }
        } else {
            Err(anyhow!("Could not find package metadata."))
        }
    }

    fn get_artifact<'a>(&mut self, index: usize) -> Option<ZipFile> {
        let artifact = self.archive.by_index(index);
        if let Ok(artifact) = artifact {
            Some(artifact)
        } else {
            None
        }
    }

    /// Get the metadata header of the package.
    pub fn metadata(&self) -> &PackageMetadata {
        &self.metadata
    }

    /// Get the ZipFile for a wasm binary file.
    pub fn get_wasm(&mut self, path: &Path) -> Option<ZipFile> {
        let index = self.wasm.get(path);

        if let Some(index) = index {
            let index = *index;
            self.get_artifact(index)
        } else {
            None
        }
    }

    /// Provides an iterator of keys of each wasm resource.
    pub fn wasm_iterator(&self) -> std::collections::hash_map::Keys<PathBuf, usize> {
        self.wasm.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn just_load() {
        let file = std::fs::File::open("../../example_mod/target/example_mod.zip").unwrap();
        let mut package = PackageFile::load(std::io::BufReader::new(file)).unwrap();

        package.get_wasm(&PathBuf::from("test_ui.wasm")).unwrap();
    }

    // TODO test for a package containing a bad path.
}
