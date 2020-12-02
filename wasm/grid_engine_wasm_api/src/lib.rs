// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! This is a library used to make user content for the Grid Engine.

#![warn(missing_docs)]

#[cfg(not(target_arch = "wasm32"))]
compile_error!("You are using the wrong compiler target. See the readme for details on how to fix that.");

#[link(wasm_import_module = "grid_api")]
extern "C" {
    fn __register_event_type(type_id: u32, name: *const u8, name_len: usize);
}

#[no_mangle]
extern "C" fn __entry_point() {
    register_event_type(5, "Event type name");
}

/// Register an event type that can be processed by an entity.
pub fn register_event_type(type_id: u32, name: &str) {
    unsafe {
        __register_event_type(type_id, name.as_bytes().as_ptr(), name.len());
    }
}
