use spirv_builder::SpirvBuilder;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Build shader code.
    SpirvBuilder::new("./gpu_code", "spirv-unknown-vulkan1.0").build()?;

    // On Windows we need to static link the standard C++ library.
    let target = std::env::var("TARGET").unwrap();
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=static=gcc");
        println!("cargo:rustc-link-lib=static=stdc++");
    }

    Ok(())
}
