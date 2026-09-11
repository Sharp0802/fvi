
struct Rect {
    pos   : vec2f,
    size  : vec2f,
    color : vec4f,
    tex   : u32,
    radius: f32,
    border_radius: f32,
    border_stroke: f32,
    border_color : vec4f,
};

struct View {
    size : vec2u,
    scale: f32,
    _pad : u32,
};
