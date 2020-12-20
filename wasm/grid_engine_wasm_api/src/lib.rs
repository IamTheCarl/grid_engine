// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! This is a library used to make user content for the Grid Engine.

#![warn(missing_docs)]

#[cfg(not(target_arch = "wasm32"))]
compile_error!("You are using the wrong compiler target. See the readme for details on how to fix that.");

use core::panic::PanicInfo;
use log::{Level, LevelFilter, Metadata, Record};
pub use proc_macros::*;

// Functions provided by the host.
#[link(wasm_import_module = "grid_api")]
extern "C" {
    fn __log_message(level: u8, source: *const u8, source_len: usize, message: *const u8, message_len: usize);
}

// Functions provided by the user.
extern "Rust" {
    fn __user_entry_point();
    fn __get_initializer(type_id: u32) -> fn() -> Box<dyn ChunkEntity>;
}

fn panic_handler(hook: &PanicInfo) {
    let source = format!("{}", "PANIC");
    let message = format!("{}", hook);

    unsafe {
        __log_message(0, source.as_bytes().as_ptr(), source.len(), message.as_bytes().as_ptr(), message.len());
    }
}

static LOGGER: GridLogger = GridLogger;

/// The main entry point for this mod.
#[no_mangle]
extern "C" fn __entry_point() {
    std::panic::set_hook(Box::new(panic_handler));

    log::set_logger(&LOGGER).expect("Failed to set logger.");

    // TODO give the host a way to control this.
    log::set_max_level(LevelFilter::Trace);

    log::info!("Logger set.");

    // The user's entry point. Our proc_macros will make sure it has the proper signature.
    unsafe {
        __user_entry_point();
    }
}

/// The engine will call this to request that an instance of a chunk entity be created.
#[no_mangle]
extern "C" fn __spawn_chunk_entity(type_id: u32) -> u64 {
    let constructor = unsafe { __get_initializer(type_id) };
    let entity = constructor();
    let pointer = Box::into_raw(entity);

    // This is ugly, I know, but the only good and safe solution was an RFC and it got axed.
    // So what are we doing here? Well a pointer in WASM is 32 bits, and this pointer to an
    // entity here is two pointers, one to the struct, the other to the vtable. We just pack
    // both of those into a single 64 bit integer.
    // Sadly, this can break with any Rust ABI changes, but the following link makes me think it
    // will eventually be stabilized:
    // https://doc.rust-lang.org/std/raw/struct.TraitObject.html
    unsafe { std::mem::transmute::<_, u64>(pointer) }
}

#[no_mangle]
extern "C" fn __drop_chunk_entity(address: u64) {
    let pointer = unsafe { std::mem::transmute::<_, *mut dyn ChunkEntity>(address) };
    let entity = unsafe { Box::from_raw(pointer) };

    // Cool we got our entity back.
    // Now drop it. That should free up the memory.
    drop(entity);
}

/// A chunk entity that can move from chunk to chunk.
pub trait ChunkEntity {}

struct GridLogger;

impl log::Log for GridLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        // We just fixed it to true for now. Might change that later.
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = match record.level() {
                Level::Error => 0,
                Level::Warn => 1,
                Level::Info => 2,
                Level::Debug => 3,
                Level::Trace => 4,
            };

            let source = format!("{}", record.target());
            let message = format!("{}", record.args());

            unsafe {
                __log_message(level, source.as_bytes().as_ptr(), source.len(), message.as_bytes().as_ptr(), message.len());
            }
        }
    }

    fn flush(&self) {}
}
