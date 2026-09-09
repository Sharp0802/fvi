//! A shader binding generator.

use std::error::Error;

use wgsl_bindgen::*;

fn main() -> Result<(), Box<dyn Error>> {
    WgslBindgenOptionBuilder::default()
        .workspace_root("gfx")
        .add_entry_point("gfx/rect_cull.wgsl")
        .add_entry_point("gfx/rect_draw.wgsl")
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .type_map(NalgebraWgslTypeMap)
        .output("src/gfx.g.rs")
        .build()?
        .generate()?;
    Ok(())
}
