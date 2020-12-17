// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use grid_engine_wasm_api::*;

static __DYNAMIC_INITIALIZERS: [fn() -> Box<dyn DynamicEntity>; 1] = [TestDynamicEntity::initialize];

#[no_mangle]
fn __get_initializer(type_id: u32) -> fn() -> Box<dyn DynamicEntity> {
    assert!((type_id as usize) < __DYNAMIC_INITIALIZERS.len());
    __DYNAMIC_INITIALIZERS[type_id as usize]
}

#[entry_point]
fn init() {
    register_event_type(0, "TestEvent0");
    register_event_type(1, "TestEvent1");
    register_event_type(2, "TestEvent2");
    register_event_type(3, "TestEvent3");
    register_event_type(4, "TestEvent4");
}

struct TestDynamicEntity;

impl TestDynamicEntity {
    fn initialize() -> Box<dyn DynamicEntity> {
        Box::new(TestDynamicEntity)
    }
}

impl DynamicEntity for TestDynamicEntity {}
