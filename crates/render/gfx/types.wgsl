#define_import_path types;

struct Rect {
    color   : vec4f,
    center  : vec2f,
    halfsize: vec2f,
    radius  : f32,
    z_order : u32,
    texture : u32,
    sampler : u32,
    mask    : u32,
    _pad    : array<u32, 3>,
};
