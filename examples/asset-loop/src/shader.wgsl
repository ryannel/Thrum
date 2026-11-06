// G5 asset-loop viewer: one cooked meshlet mesh with a cooked texture, plus a slice through a
// cooked density volume. Reversed-Z depth.

struct Scene {
    view_projection: mat4x4<f32>,
    // xyz: unit direction toward the light; w: ambient.
    light: vec4<f32>,
    // x: volume slice depth in [0, 1].
    slice: vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var density: texture_3d<f32>;
@group(1) @binding(2) var linear_sampler: sampler;

struct MeshOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

// Octahedral decode (Cigolle et al. 2014), matching the cooker's encoder.
fn octahedral_decode(e: vec2<f32>) -> vec3<f32> {
    var n = vec3<f32>(e.x, e.y, 1.0 - abs(e.x) - abs(e.y));
    let t = max(-n.z, 0.0);
    n.x += select(t, -t, n.x >= 0.0);
    n.y += select(t, -t, n.y >= 0.0);
    return normalize(n);
}

@vertex
fn mesh_vertex(@location(0) position: vec3<f32>, @location(1) normal_oct: vec2<f32>, @location(2) uv: vec2<f32>) -> MeshOut {
    var out: MeshOut;
    out.clip = scene.view_projection * vec4<f32>(position, 1.0);
    out.normal = octahedral_decode(normal_oct);
    out.uv = uv;
    return out;
}

@fragment
fn mesh_fragment(in: MeshOut) -> @location(0) vec4<f32> {
    let base = textureSample(albedo, linear_sampler, in.uv).rgb;
    let diffuse = max(dot(normalize(in.normal), scene.light.xyz), 0.0);
    return vec4<f32>(base * (scene.light.w + diffuse), 1.0);
}

struct SliceOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn slice_vertex(@builtin(vertex_index) index: u32) -> SliceOut {
    let corner = vec2<f32>(f32(index & 1u), f32((index >> 1u) & 1u));
    var out: SliceOut;
    out.clip = vec4<f32>(0.5 + corner.x * 0.45, 0.5 + corner.y * 0.45, 1.0, 1.0);
    out.uv = vec2<f32>(corner.x, 1.0 - corner.y);
    return out;
}

@fragment
fn slice_fragment(in: SliceOut) -> @location(0) vec4<f32> {
    let d = textureSampleLevel(density, linear_sampler, vec3<f32>(in.uv, scene.slice.x), 0.0).r;
    return vec4<f32>(mix(vec3<f32>(0.18, 0.32, 0.55), vec3<f32>(1.0), d), 1.0);
}
