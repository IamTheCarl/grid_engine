
//! Client for Grid Locked engine

#![warn(missing_docs)]

use jemallocator::Jemalloc;

// Use a global allocator that's better for threaded work.
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use vk_shader_macros::include_glsl;

static VERTEX_SHADER: &[u32] = include_glsl!("shaders/test.vert");
static FRAGMENT_SHADER: &[u32] = include_glsl!("shaders/test.frag");

use winit::{
    event::*,
    event_loop::{EventLoop, ControlFlow},
    window::{Window, WindowBuilder},
};

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .unwrap(); // TODO no unwrap.

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input,
                    ..
                } => {
                    match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                }
                _ => {}
            }
            _ => {}
        }
    });
}
