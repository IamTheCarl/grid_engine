// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! This is a library used to make user content for the Grid Engine.

#![warn(missing_docs)]

#[cfg(not(target_arch = "wasm32"))]
compile_error!("You are using the wrong compiler target. See the readme for details on how to fix that.");

pub mod components;
