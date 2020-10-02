//! Client for Grid Locked engine

#![warn(missing_docs)]

use jemallocator::Jemalloc;

// Use a global allocator that's better for threaded work.
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use native_dialog::{Dialog, MessageAlert, MessageType};

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod client;
use client::Client;

fn main() {
    let result = trampoline();

    if let Err(error) = result {
        // Okay, something must have gone wrong during startup or shutdown.
        // First we log it.
        log::error!("Error setting up client: {}", error);

        // Now attempt to show it in a window.
        let dialog = MessageAlert {
            title: "Critical Error",
            text: &format!("Failed to setup client: {}", error),
            typ: MessageType::Error,
        };
        let result = dialog.show();

        if let Err(error) = result {
            // If that failed too, report it too.
            log::error!("Error while reporting error: {}", error);
        }
    }
}

/// A function that generally catches errors from the client setup so that they can be properly handled
/// and displayed to the user.
fn trampoline() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let event_loop = EventLoop::new();

    // These are the only two things that can fail.
    let window = WindowBuilder::new().build(&event_loop)?;
    let mut client = Client::create_with_window(&window)?;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), .. } => {
                        *control_flow = ControlFlow::Exit
                    }
                    _ => {}
                },
                _ => {}
            },

            Event::RedrawRequested(_) => {
                client.update();
                client.render();
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }

            _ => {}
        }
    });
}
