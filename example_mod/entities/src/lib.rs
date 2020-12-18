// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use grid_engine_wasm_api::*;

dynamic_entities!([TestDynamicEntity1::initialize, TestDynamicEntity2::initialize]);

#[entry_point]
fn init() {
    register_event_type(0, "TestEvent0");
    register_event_type(1, "TestEvent1");
    register_event_type(2, "TestEvent2");
    register_event_type(3, "TestEvent3");
    register_event_type(4, "TestEvent4");
}

struct TestDynamicEntity1;

impl TestDynamicEntity1 {
    fn initialize() -> Box<dyn DynamicEntity> {
        Box::new(TestDynamicEntity1)
    }
}

impl DynamicEntity for TestDynamicEntity1 {}

struct TestDynamicEntity2;

impl TestDynamicEntity2 {
    fn initialize() -> Box<dyn DynamicEntity> {
        Box::new(TestDynamicEntity2)
    }
}

impl DynamicEntity for TestDynamicEntity2 {}
