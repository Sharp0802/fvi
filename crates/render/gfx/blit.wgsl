
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let pos = array(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );

    return vec4<f32>(pos[i], 0.0, 1.0);
}

@group(0) @binding(0) var tex: texture_2d<f32>;

@fragment
fn fs_main(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    let coord = vec2u(pos.xy);
    return textureLoad(tex, coord, 0);
}
