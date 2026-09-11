//! A shader binding generator.

use std::error::Error;

use wgsl_bindgen::*;

fn main() -> Result<(), Box<dyn Error>> {
    WgslBindgenOptionBuilder::default()
        .workspace_root("gfx")
        .ir_capabilities(
            WgslShaderIrCapabilities::default()
                | WgslShaderIrCapabilities::TEXTURE_AND_SAMPLER_BINDING_ARRAY
                | WgslShaderIrCapabilities::TEXTURE_AND_SAMPLER_BINDING_ARRAY_NON_UNIFORM_INDEXING,
        )
        .add_entry_point("gfx/blit.wgsl")
        .add_entry_point("gfx/draw.wgsl")
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .type_map(NalgebraWgslTypeMap)
        .output("src/gfx.g.rs")
        .build()?
        .generate()?;
    Ok(())
}
