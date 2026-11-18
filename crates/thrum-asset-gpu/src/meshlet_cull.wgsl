// Shared by the indexed compute cull and the object stage.
struct Bounds {
    center: vec3<f32>,
    radius: f32,
    cone_apex: vec3<f32>,
    cone_cutoff: f32,
    cone_axis: vec3<f32>,
    reserved: f32,
}

fn outside(plane: vec4<f32>, center: vec3<f32>, radius: f32) -> bool {
    return dot(plane.xyz, center) + plane.w < -radius * length(plane.xyz);
}

fn meshlet_frustum_rejection(
    b: Bounds, model: mat4x4<f32>, view_proj: mat4x4<f32>, scale: f32
) -> u32 {
    let center = (model * vec4(b.center, 1.0)).xyz;
    let radius = b.radius * scale;
    let m = transpose(view_proj);
    // WebGPU clips against -w <= x,y <= w and 0 <= z <= w. In reverse-Z,
    // z <= w is the near plane and z >= 0 is the far plane.
    if outside(m[3] + m[0], center, radius) { return 1u; }
    if outside(m[3] - m[0], center, radius) { return 2u; }
    if outside(m[3] + m[1], center, radius) { return 3u; }
    if outside(m[3] - m[1], center, radius) { return 4u; }
    if outside(m[3] - m[2], center, radius) { return 5u; }
    if outside(m[2], center, radius) { return 6u; }
    return 0u;
}

fn meshlet_rejection(
    b: Bounds, model: mat4x4<f32>, view_proj: mat4x4<f32>, camera_scale: vec4<f32>
) -> u32 {
    let frustum = meshlet_frustum_rejection(b, model, view_proj, camera_scale.w);
    if frustum != 0u { return frustum; }
    if b.cone_cutoff < 1.0 {
        let apex = (model * vec4(b.cone_apex, 1.0)).xyz;
        let axis = normalize((model * vec4(b.cone_axis, 0.0)).xyz);
        if !(dot(normalize(apex - camera_scale.xyz), axis) < b.cone_cutoff) { return 7u; }
    }
    return 0u;
}
