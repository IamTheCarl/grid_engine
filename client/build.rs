use std::{path::Path, env, error::Error, process::{Command, Stdio}};

fn main() -> Result<(), Box<dyn Error>> {
    // On Windows we need to static link the standard C++ library.
    let target = std::env::var("TARGET").unwrap();
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=static=gcc");
        println!("cargo:rustc-link-lib=static=stdc++");
    }

    let builder_crate = Path::new(env::current_dir()?.as_path()).join("gpu_build");
    println!("cargo:rerun-if-changed={}/src", builder_crate.as_os_str().to_str().expect("Could not represent builder crate path."));

    // Run the builder crate for GPU code.
    let output = Command::new("cargo")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("CARGO_PROFILE_RELEASE_DEBUG")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        // Due to issues such as https://github.com/rust-lang/rust/issues/78210
        // nuke rustflags since they are (generally) used in cross compilation
        // scenarios, but we only build the shader builder for the HOST. If this
        // ends up being a problem we might need to more surgically edit RUSTFLAGS
        // instead
        .env_remove("RUSTFLAGS")
        .arg("run")
        .arg("--release")
        .arg("--")
        .current_dir(builder_crate)
        .output()
        .expect("Failed to execute builder process.");

    if output.status.success() {
        print!("{}", String::from_utf8(output.stdout)?);
    } else {
        panic!("Failed to build shader code: {}", String::from_utf8(output.stderr)?);
    }

    Ok(())
}
