#import types::Rect;

struct IndirectArgs {
    index_count   : u32,
    instance_count: atomic<u32>,
    first_index   : u32,
    base_vertex   : i32,
    first_instance: u32,
};

@group(0) @binding(0) var<storage, read_write> indirect: IndirectArgs;
@group(1) @binding(0) var<storage, read_write> rects: array<Rect>;
@group(1) @binding(1) var<storage, read_write> visibles: array<u32>;

const BIT_ALIVE: u32 = 0x00000001;

@compute
@workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) tid: vec3u) {
    let index = tid.x;
    if index >= arrayLength(&rects) { return; }

    if (rects[index].mask & BIT_ALIVE) == BIT_ALIVE {
        let mapped = atomicAdd(&indirect.instance_count, 1u);
        visibles[mapped] = index;
    }

    rects[index].mask &= ~BIT_ALIVE;
}
