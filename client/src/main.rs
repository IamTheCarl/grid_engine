// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Client for Grid Locked engine

#![warn(missing_docs)]

use native_dialog::{MessageDialog, MessageType};

use anyhow::{Context, Result};
use winit::{dpi, event::*, event_loop::ControlFlow, event_loop::EventLoop, window::Window, window::WindowBuilder};

mod client;
use client::Client;

fn main() {
    let result = trampoline();

    if let Err(error) = result {
        // Okay, something must have gone wrong during startup or shutdown.
        // First we log it.
        log::error!("Error setting up client: {:?}", error);

        // Now attempt to show it in a window.
        let message = format!("{:?}", error);
        let dialog = MessageDialog::new().set_title("Critical Error").set_text(&message).set_type(MessageType::Error);
        let result = dialog.show_confirm();

        if let Err(error) = result {
            // If that failed too, report it too.
            log::error!("Error while reporting error: {}", error);
        }
    }
}

/// Used to identify controls on the PC (this main body is for PC only)
#[derive(std::cmp::PartialEq, std::cmp::Eq, std::hash::Hash)]
enum ControlInput {
    KeyboardInput(winit::event::ScanCode),
    MouseMoveX,
    MouseMoveY,
    MouseWheel,
}

impl client::InputKey for ControlInput {}

/// A function that generally catches errors from the client setup so that they
/// can be properly handled and displayed to the user.
fn trampoline() -> Result<()> {
    env_logger::init();

    log::info!("Welcome to Grid Engine!");
    common::log_basic_system_info().context("Error logging basic system info.")?;

    let event_loop = EventLoop::new();

    // These are the only two things that can fail.
    let window = WindowBuilder::new().build(&event_loop).context("Error creating window.")?;
    let our_window_id = window.id();
    let mut client: Client<ControlInput> = Client::create_with_window(window).context("Error setting up graphics system.")?;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { ref event, window_id } if window_id == our_window_id => match event {
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), .. } => {
                        // TODO this should be passed as a special event.
                    }
                    _ => {}
                },
                WindowEvent::MouseInput { device_id, state, button, .. } => {}
                _ => {}
            },
            _ => {}
        }
        let new_flow = client.process_event(&event);
        if let Some(new_flow) = new_flow {
            *control_flow = new_flow;
        }
    });
}
