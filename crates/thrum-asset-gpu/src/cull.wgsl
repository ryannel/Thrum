// Per-meshlet frustum and normal-cone culling; writes one indexed indirect draw per meshlet.

struct Cull {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    // xyz: camera in world space, w: largest axis scale of `model`.
    camera_scale: vec4<f32>,
    // x: meshlet count.
    counts: vec4<u32>,
}
struct Bounds {
    center: vec3<f32>,
    radius: f32,
    cone_apex: vec3<f32>,
    cone_cutoff: f32,
    cone_axis: vec3<f32>,
    reserved: f32,
}
@group(0) @binding(0) var<uniform> cull: Cull;
@group(0) @binding(1) var<storage, read> bounds: array<Bounds>;
@group(0) @binding(2) var<storage, read> ranges: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read_write> draws: array<u32>;
@group(0) @binding(4) var<storage, read_write> visible: atomic<u32>;

fn outside(plane: vec4<f32>, center: vec3<f32>, radius: f32) -> bool {
    return dot(plane.xyz, center) + plane.w < -radius * length(plane.xyz);
}

@compute @workgroup_size(64) fn cull_meshlets(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if index >= cull.counts.x { return; }
    let b = bounds[index];
    let center = (cull.model * vec4(b.center, 1.0)).xyz;
    let radius = b.radius * cull.camera_scale.w;
    let m = transpose(cull.view_proj);
    // Left, right, bottom, top and (reverse-Z) near planes; the far plane is at infinity.
    var keep = !(outside(m[3] + m[0], center, radius) || outside(m[3] - m[0], center, radius)
        || outside(m[3] + m[1], center, radius) || outside(m[3] - m[1], center, radius)
        || outside(m[3] - m[2], center, radius));
    if keep && b.cone_cutoff < 1.0 {
        let apex = (cull.model * vec4(b.cone_apex, 1.0)).xyz;
        let axis = normalize((cull.model * vec4(b.cone_axis, 0.0)).xyz);
        keep = dot(normalize(apex - cull.camera_scale.xyz), axis) < b.cone_cutoff;
    }
    let range = ranges[index];
    let base = index * 5u;
    draws[base] = range.y;
    draws[base + 1u] = select(0u, 1u, keep);
    draws[base + 2u] = range.x;
    draws[base + 3u] = 0u;
    draws[base + 4u] = 0u;
    if keep { atomicAdd(&visible, 1u); }
}
