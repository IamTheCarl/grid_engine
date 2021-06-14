use spirv_builder::SpirvBuilder;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Build shader code.
    SpirvBuilder::new("./gpu_code", "spirv-unknown-vulkan1.0").build()?;

    Ok(())
}
