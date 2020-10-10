// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use anyhow::Result;

fn main() {
    let result = trampoline();

    if let Err(error) = result {
        // Okay, something must have gone wrong during startup or shutdown.
        // First we log it.
        log::error!("Error setting up server: {}", error);
    }
}

/// A function that generally catches errors from the client setup so that they can be properly handled
/// and displayed to the user.
fn trampoline() -> Result<()> {
    env_logger::init();

    log::info!("Welcome to Grid Engine!");
    common::log_basic_system_info()?;

    Ok(())
}
