// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Common library used for both client and server.

#![warn(missing_docs)]

mod time;
pub use time::*;

pub mod modules;
pub mod wasm;
pub mod physics;
pub mod scheduler;

/// Writes basic system info such as the OS, memory, and the processor used.
/// Does not log anything about the GPU.
pub fn log_basic_system_info() -> Result<(), Box<dyn std::error::Error>> {
    let os = os_info::get();

    log::info!("OS Info: {}", os);
    log::info!("Target Arch: {}", std::env::consts::ARCH);

    let cpu_num = sys_info::cpu_num()?;

    log::info!("Hardware Threads: {}", cpu_num);

    let cpu_speed = sys_info::cpu_speed()?;

    log::info!("CPU Speed: {}MHz", cpu_speed);

    let mem_info = sys_info::mem_info()?;

    log::info!("Physical Memory: {}Mb", mem_info.total / 1000);
    log::info!("Swap Memory: {}Mb", mem_info.swap_total / 1000);

    Ok(())
}
