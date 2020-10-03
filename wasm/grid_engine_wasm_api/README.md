This package is in extremely early development.
Consider this more of a placeholder than an actual package.

# Setting up your dev environment:
First you must install the web assembly development toolchain for rust.
Do so with the following command in a terminal:
```
rustup target add wasm32-wasi
```

That's it. You're ready to go.

# Creating your project

Your project including this library must be built as a web assembly target. There are two ways to do this.
First: Just pass the target to cargo manually.
```
cargo build --target wasm32-wasi
```

Second: Provide a config file `.cargo/config.toml` that provides the target.
The content of that config file should be as so.
```
[build]
target = "wasm32-wasi"
```