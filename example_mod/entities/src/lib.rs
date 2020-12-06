// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use grid_engine_wasm_api::entry_point;
use grid_engine_wasm_api::register_event_type;

#[entry_point]
fn init() {
    register_event_type(0, "TestEvent0");
    register_event_type(1, "TestEvent1");
    register_event_type(2, "TestEvent2");
    register_event_type(3, "TestEvent3");
    register_event_type(4, "TestEvent4");
}
