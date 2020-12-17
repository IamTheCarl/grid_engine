// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! This is a library used to make user content for the Grid Engine.

#![warn(missing_docs)]

#[cfg(not(target_arch = "wasm32"))]
compile_error!("You are using the wrong compiler target. See the readme for details on how to fix that.");

pub use proc_macros::*;

// Functions provided by the host.
#[link(wasm_import_module = "grid_api")]
extern "C" {
    fn __register_event_type(type_id: u32, name: *const u8, name_len: usize);
}

// Functions provided by the user.
extern "Rust" {
    fn __user_entry_point();
    fn __get_initializer(type_id: u32) -> fn() -> Box<dyn DynamicEntity>;
}

/// Register an event type that can be processed by an entity.
pub fn register_event_type(type_id: u32, name: &str) {
    unsafe {
        __register_event_type(type_id, name.as_bytes().as_ptr(), name.len());
    }
}

/// The main entry point for this mod.
#[no_mangle]
extern "C" fn __entry_point() {
    // The user's entry point. Our proc_macros will make sure it has this signature.
    unsafe {
        __user_entry_point();
    }
}

/// The engine will call this to request that an instance of a dynamic entity be created.
#[no_mangle]
extern "C" fn __spawn_dynamic_entity(type_id: u32) -> u64 {
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
extern "C" fn __drop_dynamic_entity(address: u64) {
    let pointer = unsafe { std::mem::transmute::<_, *mut dyn DynamicEntity>(address) };
    let entity = unsafe { Box::from_raw(pointer) };

    // Cool we got our entity back.
    // Now drop it. That should free up the memory.
    drop(entity);
}

/// A dynamic entity that can move from chunk to chunk.
pub trait DynamicEntity {}
