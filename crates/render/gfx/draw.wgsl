enable wgpu_binding_array;

#import types::{Rect, View};

struct Vert {
    @builtin(position)              pos : vec4f,
    @location(1)                    uv  : vec2f,
    @location(2) @interpolate(flat) inst: u32,
};

@group(0) @binding(0) var<storage, read> rects: array<Rect>;
@group(1) @binding(0) var                texs : binding_array<texture_2d<f32>>;
@group(1) @binding(1) var                samp : sampler;
@group(2) @binding(0) var<uniform>       view : View;

fn sd_round_rect(p: vec2f, b: vec2f, radius: f32) -> f32 {
    let r = clamp(radius, 0.0, min(b.x, b.y));
    let q = abs(p) - b + r;
    return length(max(q, vec2f(0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sdf_cov(sd: f32) -> f32 {
    return clamp(0.5 - sd / max(fwidth(sd), 1e-6), 0.0, 1.0);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vert: u32,
    @builtin(instance_index) inst: u32,
) -> Vert {
    let uvs = array(
        vec2f(0, 0),
        vec2f(1, 0),
        vec2f(0, 1),
        vec2f(1, 1),
    );

    let rect = rects[inst];
    let uv = uvs[vert];
    let p = (rect.pos + rect.size * uv) * view.scale;
    let size = vec2f(view.size);

    var out: Vert;
    out.pos = vec4f(
        p.x * 2.0 / size.x - 1.0,
        1.0 - p.y * 2.0 / size.y,
        0.0,
        1.0,
    );
    out.uv = uv;
    out.inst = inst;
    return out;
}

@fragment
fn fs_main(vert: Vert) -> @location(0) vec4f {
    let rect = rects[vert.inst];
    let half = rect.size * 0.5;
    let p = (vert.uv - 0.5) * rect.size;

    let fill_cov = sdf_cov(sd_round_rect(p, half, rect.radius));
    let fill = rect.color * textureSample(texs[rect.tex], samp, vert.uv);

    let stroke = max(rect.border_stroke, 0.0);
    let outer_cov = sdf_cov(sd_round_rect(p, half, rect.border_radius));

    let inner_half = max(half - stroke, vec2f(0));
    let inner_radius = max(rect.border_radius - stroke, 0.0);

    var inner_cov = sdf_cov(sd_round_rect(p, inner_half, inner_radius));
    if stroke >= min(half.x, half.y) {
        inner_cov = 0.0;
    }

    var border_cov = clamp(outer_cov - inner_cov, 0.0, 1.0);
    if stroke <= 0.0 {
        border_cov = 0.0;
    }

    let fa = fill.a * fill_cov;
    let ba = rect.border_color.a * border_cov;
    let a = ba + fa * (1.0 - ba);

    let rgb = (
        rect.border_color.rgb * ba +
        fill.rgb * fa * (1.0 - ba)
    ) / max(a, 1e-6);

    return vec4f(rgb, a);
}
