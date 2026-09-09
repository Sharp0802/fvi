#import types::Rect;

struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(1) @interpolate(flat) color: vec4f,
    @location(2) @interpolate(flat) center: vec2f,
    @location(3) @interpolate(flat) half_size: vec2f,
    @location(4) @interpolate(flat) radius: f32,
};

@group(0) @binding(0) var<uniform> scale: f32;
@group(1) @binding(0) var<storage, read> rects: array<Rect>;
@group(1) @binding(1) var<storage, read> visibles: array<u32>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance: u32,
) -> VertexOutput {
    const uvs = array(
        vec2f( 1,  1),
        vec2f(-1,  1),
        vec2f(-1, -1),
        vec2f( 1, -1),
    );

    let id = visibles[instance];
    let rect = rects[id];

    let uv = uvs[vertex];
    let center = rect.center * scale;
    let half_size = rect.halfsize * scale;
    let pos = center + half_size * uv;

    var output: VertexOutput;
    output.pos = vec4f(pos, 0, f32(rect.z_order));
    output.center = center;
    output.half_size = half_size;
    output.radius = rect.radius;
    return output;
}

fn rect_sdf(
    pos: vec2f,
    center: vec2f,
    half_size: vec2f,
    radius: f32,
) -> f32 {
    let d2 = abs(center - pos)
        - half_size
        + vec2f(radius);
    return min(max(d2.x, d2.y), 0)
        + length(vec2f(max(d2.x, 0), max(d2.y, 0)))
        - radius;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let dist = rect_sdf(input.pos.xy, input.center, input.half_size, input.radius);
    let factor = 1 - smoothstep(0., 2, dist);
    let premul = vec4f(input.color.rgb, 1) * input.color.a;
    return premul * factor;
}
