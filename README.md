Grid Locked is a game I've wanted to make since I was in highschool.

Imagine Minecraft and Factorio had a baby, except you can also build flying ships as well.

This is currently a very early work in progress.
I'm still working out details.

# Organization

This project is split into three major parts.

## Native

The native parts of the project are the engine itself. They are compiled for each platform being targeted.

### Common
This is actually the majority of the engine. It contains all the parts that are common between the headless server and the client.

### Client

This is the application that players run on their desktop computers. It contains graphics and IO libraries that are not useful to a headless server.

### Server

The headless server application used for dedicated servers. It is significantly different from the client and really only meant to run on Linux systems. As a result, it is not compatable with the goals of the client.

## Wasm

Wasm contains the portable parts of this project that are compiled once to webassembly to run on all platforms under the native parts of this project.

### grid_engine_wasm_api

Maybe not the best name for the library, but at least it's clear. This is a library hosted on [crates.io](https://crates.io/crates/grid_engine_wasm_api) meant to assist in the development of your own content for this engine.

## example_mod

This is an example of most features used by the modding API. It isn't really meant to serve as a template for your projects (a template repository will be provided in the future) but rather as an example and proof of practicality for common use cases of the API.

# Building the project

This project is written in Rust, and like any good modern Rust project, it uses cargo as the build system of choice.
To install cargo you must first install the rust toolchain manager [rustup](https://rustup.rs/). Once that's done, you're ready to build this project.

This project is split into two halves, the native side and the wasm side. The wasm side is used for portable things, like mods and game content. The native side is used for non-portable things, such as the engine itself and the headless server. These two sets of projects have been grouped up into work spaces to make for significant savings in build time and hard drive space usage for the developer. The two halves must be separate workspaces though, because cargo cannot have multiple build targets in the same project. Each workspace must be built individually.

The simple solution is to just go into each workspace's directory and run `cargo build` from there.

The fancy automated solution is to install [cargo-make](https://crates.io/crates/cargo-make#installation). Then just run `cargo make build` from the root of this project.