//! A shader binding generator.

use std::error::Error;

use wgsl_bindgen::{RustWgslTypeMap, WgslBindgenOptionBuilder, WgslTypeSerializeStrategy};

fn main() -> Result<(), Box<dyn Error>> {
    WgslBindgenOptionBuilder::default()
        .workspace_root("gfx")
        .add_entry_point("gfx/cull.wgsl")
        .add_entry_point("gfx/draw.wgsl")
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .type_map(RustWgslTypeMap)
        .output("src/gfx.g.rs")
        .build()?
        .generate()?;
    Ok(())
}
