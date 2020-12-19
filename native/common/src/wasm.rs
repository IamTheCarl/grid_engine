// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Management of web assembly assets.

use crate::modules::PackageFile;
use anyhow::{Context, Result};
use log::Level;
use std::ffi::c_void;
use std::io::{Read, Seek};
use std::path::PathBuf;
use wasmer_runtime::{func, imports, Array, Ctx, Func, Instance, WasmPtr};

struct ModData {}

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
            wasm.read_to_end(&mut wasm_binary)?;
        }

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
                    // TODO include the mod's name in this message.
                    log::log!(level, "{}: {}", source, message);
                })
            }
        };

        // We will need to create multiple instances from this modules, so store it separate from the modules.
        let module = wasmer_runtime::compile(&wasm_binary)?;
        let wasm_instance = module.instantiate(&imports).unwrap();

        // We have to pin this so it won't get moved in memory and mess up our pointers.
        let mut wasm_file = WasmFile { wasm_instance };
        let root_context = wasm_file.wasm_instance.context_mut();

        let user_data: *mut c_void = Box::into_raw(Box::new(ModData {})) as *mut c_void;
        root_context.data = user_data;

        let __spawn_dynamic_entity: Func<u32, u64> = wasm_file.wasm_instance.exports.get("__spawn_dynamic_entity")?;
        let __drop_dynamic_entity: Func<u64, ()> = wasm_file.wasm_instance.exports.get("__drop_dynamic_entity")?;

        let pointer = __spawn_dynamic_entity.call(0).unwrap();
        println!("Entity memory address in WASM: {:x}", pointer);
        __drop_dynamic_entity.call(pointer).unwrap();

        wasm_file.run_entry_point()?;

        Ok(wasm_file)
    }

    fn run_entry_point(&self) -> Result<()> {
        let __entry_point: Func<(), ()> = self.wasm_instance.exports.get("__entry_point")?;

        __entry_point.call().unwrap();

        Ok(())
    }

    fn get_mod_data(&self) -> &ModData {
        let root_context = self.wasm_instance.context();
        let mod_data = unsafe { root_context.data.cast::<ModData>().as_ref() }.expect("Internal mod data was not initialized.");
        mod_data
    }
    // pub fn spawn_dynamic_entity(&self, type_id: u32) {

    // }
}

impl Drop for WasmFile {
    fn drop(&mut self) {
        // We must drop the user data.
        let (_memory, user_data) = unsafe { self.wasm_instance.context_mut().memory_and_data_mut::<ModData>(0) };
        drop(user_data);
    }
}

pub struct WasmDynamicEntity {}

impl WasmDynamicEntity {}
