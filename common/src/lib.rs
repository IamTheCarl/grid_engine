// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Common library used for both client and server.

#![warn(missing_docs)]

mod time;
pub use time::*;

pub mod physics;
pub mod scheduler;
