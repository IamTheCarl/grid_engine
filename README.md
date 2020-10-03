Grid Locked is a game I've wanted to make since I was in highschool.

Imagine Minecraft and Factorio had a baby, except you can also build flying ships as well.

This is currently a very early work in progress.
I'm still working out details of the road map.

# Building the project

This project is built in Rust, and like any good modern Rust project, it uses cargo as the build system of choice.
To install cargo you must first install the rust toolchain manager [rustup](https://rustup.rs/). Once that's done, you're ready to build this project.

Simply typing `cargo build` will build the common library, the client, and the server. It will not build the `grid_engine_wasm_api`. This is because that sub project has special build requirements relating to the fact that it is cross compiled to web assembly. Sadly, a cargo workspace doesn't respect the build target settings of sub projects.

The simple solution is to just go into that project's directory and run `cargo build` from there.

The fancy automated solution is to install [cargo-make](https://crates.io/crates/cargo-make#installation). Then just run `cargo make build` from the root of this project.