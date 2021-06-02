Grid Locked is a game I've wanted to make since I was in highschool.

Imagine Minecraft and Factorio had a baby, except you can also build flying ships as well.

This is currently a very early work in progress.
I'm still working out details.

# Organization

This project is split into three major parts.

## Common
This is actually the majority of the engine. It contains all the parts that are common between the headless server and the client.

## Client

This is the application that players run on their desktop computers. It contains graphics and IO libraries that are not useful to a headless server.

## Server

The headless server application used for dedicated servers. It is significantly different from the client and really only meant to run on Linux systems, perhaps even only within a Docker container. As a result, it is not compatible with the goals of the client.

# Building the project

This project is written in Rust, and like any good modern Rust project, it uses cargo as the build system of choice.
To install cargo you must first install the rust toolchain manager [rustup](https://rustup.rs/). Once that's done, you're ready to build this project.

This project uses nightly Rust to build. A toolchain file has been left in the root of the project, so when you go to build it will automatically
download and set that up for you.

I do make an effort to avoid C dependencies in this project, so only the Rust compiler should be needed.
Still it's easy for them to slip in. If one does slip in, just install build essentials and you'll be good to go.