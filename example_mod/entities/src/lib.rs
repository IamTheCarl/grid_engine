// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use grid_engine_wasm_api::*;

chunk_entities!([("TestChunkEntity1", TestChunkEntity1::initialize), ("TestChunkEntity2", TestChunkEntity2::initialize)]);

#[entry_point]
fn init() {}

struct TestChunkEntity1;

impl TestChunkEntity1 {
    fn initialize() -> Box<dyn ChunkEntity> {
        log::info!("Spawn entity of type 1.");
        Box::new(TestChunkEntity1)
    }
}

impl ChunkEntity for TestChunkEntity1 {}

impl Drop for TestChunkEntity1 {
    fn drop(&mut self) {
        log::info!("Dropped entity of type 1.");
    }
}

struct TestChunkEntity2;

impl TestChunkEntity2 {
    fn initialize() -> Box<dyn ChunkEntity> {
        log::info!("Spawn entity of type 2.");
        Box::new(TestChunkEntity2)
    }
}

impl ChunkEntity for TestChunkEntity2 {}

impl Drop for TestChunkEntity2 {
    fn drop(&mut self) {
        log::info!("Dropped entity of type 2.");
    }
}
