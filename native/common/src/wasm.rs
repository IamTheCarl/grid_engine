// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Management of web assembly assets.

use crate::modules::PackageFile;
use anyhow::{anyhow, Context, Result};
use log::Level;
use std::collections::HashMap;
use std::ffi::c_void;
use std::io::{Read, Seek};
use std::path::PathBuf;
use wasmer_runtime::{func, imports, Array, Ctx, Func, Instance, WasmPtr};

fn process_wasm_result<T, E>(result: Result<T, E>) -> Result<T>
where
    E: std::fmt::Display,
{
    match result {
        Ok(t) => Ok(t),
        Err(error) => Err(anyhow!("WASM Error: {}", error)),
    }
}

/// The TypeID of a chunk entity, used to spawn and identify the type.
pub struct ChunkEntityTypeID {
    type_id: u32,
}

struct ModData {
    name: String,
    chunk_entity_ids: HashMap<String, u32>,
    chunk_entity_names: Vec<String>,
}

/// Represents a web assembly file in a module.
pub struct WasmFile {
    wasm_instance: Instance,
}

impl WasmFile {
    /// Loads the web assembly file from the module.
    pub fn load<R: Read + Seek>(package: &mut PackageFile<R>, file_name: &str) -> Result<WasmFile> {
        let mut wasm_binary = Vec::new();

        {
            // Get a reader for the file.
            let path = PathBuf::from(file_name);
            let mut wasm =
                package.get_wasm(&path).context("Error while fetching wasm file from package: File does not exist.")?;

            // Unpack it into memory.
            wasm.read_to_end(&mut wasm_binary).context("Error while reading web assembly file.")?;
        }

        // We provide the mod with an API to communicate with us through.
        let imports = imports! {
            "grid_api" => {
                "__log_message" => func!(move |ctx: &mut Ctx, level: u8, source: WasmPtr<u8, Array>, source_len: u32, message: WasmPtr<u8, Array>, message_len: u32| {
                    let (memory, mod_data) = unsafe { ctx.memory_and_data_mut::<ModData>(0) };
                    let level = match level {
                        0 => Level::Error,
                        1 => Level::Warn,
                        2 => Level::Info,
                        3 => Level::Debug,
                        4 => Level::Trace,
                        _ => {
                            log::warn!("Log message received with invalid log level. Assuming warning for log level.");
                            Level::Warn
                        }
                    };
                    let source = source.get_utf8_string(memory, source_len).expect("Could not fetch memory.");
                    let message = message.get_utf8_string(memory, message_len).expect("Could not fetch memory.");
                    log::log!(level, "{} - {}: {}", mod_data.name, source, message);
                }),
                "__register_chunk_entity_initializer_name" => func!(move |ctx: &mut Ctx, name: WasmPtr<u8, Array>, name_len: u32| {
                    let (memory, mod_data) = unsafe { ctx.memory_and_data_mut::<ModData>(0) };
                    let name = name.get_utf8_string(memory, name_len).expect("Could not fetch memory.");
                    let is_duplicate_name = mod_data.chunk_entity_ids.insert(String::from(name), mod_data.chunk_entity_ids.len() as u32).is_some();
                    mod_data.chunk_entity_names.push(String::from(name));

                    log::info!("Registered chunk entity {} in the {} mod.", name, mod_data.name);
                    if is_duplicate_name {
                        // TODO should this be an error?
                        log::warn!("Two chunk entities in the {} mod share the name {}. \
                        When this happens, the second entity to be given this name is used.", mod_data.name, name);
                    }
                })
            }
        };

        // We will need to create multiple instances from this modules, so store it separate from the modules.
        let module = wasmer_runtime::compile(&wasm_binary).context("Error compiling web assembly.")?;
        let wasm_instance = process_wasm_result(module.instantiate(&imports)).context("Error instantiating WASM instance.1")?;

        // We have to pin this so it won't get moved in memory and mess up our pointers.
        let mut wasm_file = WasmFile { wasm_instance };
        let root_context = wasm_file.wasm_instance.context_mut();

        // TODO this isn't the best name. Should probably get the name from a config in the mod.
        let user_data: *mut c_void = Box::into_raw(Box::new(ModData {
            name: String::from(file_name),
            chunk_entity_ids: HashMap::new(),
            chunk_entity_names: Vec::new(),
        })) as *mut c_void;
        root_context.data = user_data;

        wasm_file.run_entry_point().context("Error while running mod's entry point.")?;

        Ok(wasm_file)
    }

    fn run_entry_point(&self) -> Result<()> {
        let __entry_point: Func<(), ()> = self
            .wasm_instance
            .exports
            .get("__entry_point")
            .context("Error finding mod's entry point. Did you remember to create an init function?")?;
        process_wasm_result(__entry_point.call())?;

        Ok(())
    }

    fn get_mod_data(&self) -> &ModData {
        let root_context = self.wasm_instance.context();
        let mod_data = unsafe { root_context.data.cast::<ModData>().as_ref() }.expect("Internal mod data was not initialized.");
        mod_data
    }

    /// Get the type ID for an entity from its name.
    pub fn get_chunk_entity_type_id(&self, name: &str) -> Option<ChunkEntityTypeID> {
        let mod_data = self.get_mod_data();
        let type_id = mod_data.chunk_entity_ids.get(name);
        if let Some(type_id) = type_id {
            Some(ChunkEntityTypeID { type_id: *type_id })
        } else {
            None
        }
    }

    /// Get the name of a type ID.
    pub fn get_chunk_entity_type_name(&self, type_id: ChunkEntityTypeID) -> &str {
        let mod_data = self.get_mod_data();
        let name = mod_data.chunk_entity_names.get(type_id.type_id as usize);
        if let Some(name) = name {
            name
        } else {
            "Invalid Type ID"
        }
    }

    /// Spawn an entity within the WASM VM.
    pub fn spawn_chunk_entity(&self, type_id: ChunkEntityTypeID) -> Result<WasmChunkEntity> {
        // FIXME fetching this function every time we run is going to induce some slowdown. See if you can fix that.
        let __spawn_chunk_entity: Func<u32, u64> = self
            .wasm_instance
            .exports
            .get("__spawn_chunk_entity")
            .context("Failed to get __spawn_chunk_entity function from wasm.")?;

        // TODO we need an abstraction for the type_id.
        let wasm_address = process_wasm_result(__spawn_chunk_entity.call(type_id.type_id))?;
        let __drop_chunk_entity: Func<u64, ()> = self.wasm_instance.exports.get("__drop_chunk_entity")?;

        Ok(WasmChunkEntity { wasm_address, __drop_chunk_entity })
    }
}

impl Drop for WasmFile {
    fn drop(&mut self) {
        // We must drop the user data.
        let (_memory, user_data) = unsafe { self.wasm_instance.context_mut().memory_and_data_mut::<ModData>(0) };
        drop(user_data);
    }
}

/// A chunk entity living in the WASM VM.
pub struct WasmChunkEntity<'a> {
    wasm_address: u64,
    __drop_chunk_entity: Func<'a, u64, ()>,
}

impl<'a> WasmChunkEntity<'a> {}

impl<'a> Drop for WasmChunkEntity<'a> {
    fn drop(&mut self) {
        let result = process_wasm_result(self.__drop_chunk_entity.call(self.wasm_address));
        if let Err(error) = result {
            log::error!("Error while deleting chunk entity from WASM VM: {}", error);
        }
    }
}
