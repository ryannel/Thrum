// Per-meshlet frustum and normal-cone culling; writes one indexed indirect draw per meshlet.

struct Cull {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    // xyz: camera in world space, w: largest axis scale of `model`.
    camera_scale: vec4<f32>,
    // x: meshlet count.
    counts: vec4<u32>,
}
@group(0) @binding(0) var<uniform> cull: Cull;
@group(0) @binding(1) var<storage, read> bounds: array<Bounds>;
@group(0) @binding(2) var<storage, read> ranges: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read_write> draws: array<u32>;
@group(0) @binding(4) var<storage, read_write> visible: atomic<u32>;

@compute @workgroup_size(64) fn cull_meshlets(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if index >= cull.counts.x { return; }
    let keep = meshlet_rejection(bounds[index], cull.model, cull.view_proj, cull.camera_scale) == 0u;
    let range = ranges[index];
    let base = index * 5u;
    draws[base] = range.y;
    draws[base + 1u] = select(0u, 1u, keep);
    draws[base + 2u] = range.x;
    draws[base + 3u] = 0u;
    draws[base + 4u] = 0u;
    if keep { atomicAdd(&visible, 1u); }
}
