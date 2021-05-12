// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

//! Client for Grid Locked engine

#![warn(missing_docs)]

use jemallocator::Jemalloc;

// Use a global allocator that's better for threaded work.
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use native_dialog::{MessageDialog, MessageType};

use anyhow::{Context, Result};
use winit::{event_loop::EventLoop, window::WindowBuilder};

// mod client;
// mod users;

// use client::Client;

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

/// A function that generally catches errors from the client setup so that they
/// can be properly handled and displayed to the user.
fn trampoline() -> Result<()> {
    env_logger::init();

    log::info!("Welcome to Grid Engine!");
    common::log_basic_system_info().context("Error logging basic system info.")?;

    let event_loop = EventLoop::new();

    // These are the only two things that can fail.
    let window = WindowBuilder::new().build(&event_loop).context("Error creating window.")?;
    // let mut client = Client::create_with_window(window).context("Error setting up graphics system.")?;

    event_loop.run(move |event, _, control_flow| {
        // let new_flow = client.process_event(&event);
        // if let Some(new_flow) = new_flow {
        //     *control_flow = new_flow;
        // }
    });
}
