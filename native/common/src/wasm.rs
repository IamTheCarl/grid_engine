// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Management of web assembly assets.

use crate::modules::PackageFile;
use anyhow::{Context, Result};
use std::io::{Read, Seek};
use std::path::PathBuf;
use wasmer_runtime::{error, func, imports, instantiate, Func};

/// Represents a webassembly file in a module.
pub struct WasmFile;

impl WasmFile {
    /// Loads the webassembly file from the module.
    pub fn load<R: Read + Seek>(package: &mut PackageFile<R>, file_name: &str) -> Result<WasmFile> {
        let mut wasm_binary = Vec::new();

        {
            // Get a reader for the file.
            let path = PathBuf::from(file_name);
            let mut wasm =
                package.get_wasm(&path).context("Error while fetching wasm file from package: File does not exist.")?;

            // Unpack it into memory.
            wasm.read_to_end(&mut wasm_binary)?;
        }

        let import_objects = imports! {
            "grid_api" => {
                "__register_event_type" => func!(|_type_id: u32, _name: u32, _name_len: u32|{
                    println!("Called!");
                })
            }
        };

        // Let's create an instance of Wasm module running in the wasmer-runtime
        let instance = instantiate(&wasm_binary, &import_objects).unwrap();

        for export in instance.exports() {
            println!("Export: {}", export.0);
        }

        let __entry_point: Func<(), ()> = instance.exports.get("__entry_point")?;

        __entry_point.call().unwrap();

        Ok(WasmFile)
    }
}
