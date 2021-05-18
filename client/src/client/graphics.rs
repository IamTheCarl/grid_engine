// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Stuff for rendering the world. It lives here in the client, and not common, because
//! the server will never make use of this.

use common::world::{Chunk, ChunkCoordinate, ChunkCoordinateEXT, ChunkIterator, GridWorld};
use nalgebra::{Isometry3, Perspective3};

/// Type for graphics computations.
pub type GraphicsVector3 = nalgebra::Vector3<f32>;

/// Data needed to render a chunk's graphics.
struct ChunkGraphicalData {
    vertex_buffer: Option<wgpu::Buffer>,
    needs_update: bool,
}

impl Default for ChunkGraphicalData {
    fn default() -> Self {
        ChunkGraphicalData { vertex_buffer: None, needs_update: true }
    }
}

/// A chunk that can be rendered by the GPU.
pub type GraphicalChunk = Chunk<ChunkGraphicalData>;

/// A version of the Grid World that can be rendered.
pub type GraphicalGridWorld = GridWorld<ChunkGraphicalData>;

pub fn render_terrain(
    world: &mut GraphicalGridWorld, chunks: ChunkIterator, device: &mut wgpu::Device, queue: &mut wgpu::Queue,
    render_pass: wgpu::RenderPass,
) {
    let mut cpu_buffer = Vec::new();

    for chunk_address in chunks {
        // TODO we could generate the meshes in parallel. I'm not sure if we should.
        if let Some(chunk) = world.get_chunk_mut(&chunk_address) {
            // We will only attempt to render chunks that actually exist.
            if chunk.user_data().needs_update {
                build_chunk_vertex_buffer(&mut cpu_buffer, chunk);

                let user_data = chunk.user_data_mut();
                let gpu_buffer = user_data.vertex_buffer.get_or_insert_with(|| {
                    device.create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        size: 0,
                        usage: wgpu::BufferUsage::VERTEX,
                        mapped_at_creation: false,
                    })
                });

                queue.write_buffer(gpu_buffer, 0, bytemuck::cast_slice(&cpu_buffer));
                user_data.needs_update = false;
            }
        }
    }
}

fn build_chunk_vertex_buffer(buffer: &mut Vec<GraphicsVector3>, chunk: &GraphicalChunk) {
    // We assume the buffer is unclean.
    buffer.clear();

    let chunk_offset = chunk.index().to_block_coordinate().cast();

    // TODO we are doing this so dumbly we just render every single block. Try and make this remove hidden faces.
    for block in chunk.iter_ideal(GraphicalChunk::range_all_blocks()) {
        // Top face.
        buffer.reserve(6);
        buffer.push(chunk_offset + GraphicsVector3::new(0.0, 0.0, 0.0));
        buffer.push(chunk_offset + GraphicsVector3::new(1.0, 0.0, 0.0));
        buffer.push(chunk_offset + GraphicsVector3::new(1.0, 0.0, 1.0));

        buffer.push(chunk_offset + GraphicsVector3::new(1.0, 0.0, 1.0));
        buffer.push(chunk_offset + GraphicsVector3::new(1.0, 0.0, 0.0));
        buffer.push(chunk_offset + GraphicsVector3::new(0.0, 0.0, 0.0));
    }
}
