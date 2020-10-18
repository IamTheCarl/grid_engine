// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Mechanisms and components revolving around what the player sees as a world.

use std::collections::HashMap;

struct World {}

struct TerrainChunkKey {
    key: u64,
}

impl TerrainChunkKey {
    fn new(x: u16, y: u16, z: u16) {}
}

struct ChunkFile {
    terrain_chunk_index: HashMap<TerrainChunkKey, usize>,
}

impl World {}

struct TerrainChunk {}

impl TerrainChunk {}

#[repr(C)]
struct TerrainBlock {}
