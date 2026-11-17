@group(0) @binding(5) var<uniform> secondary: SecondaryViewRecord;

fn large_enough(m: mat4x4<f32>, center: vec3<f32>, radius: f32) -> bool {
    if secondary.options.z <= 0.0 { return true; }
    let clip = cull.view_proj * vec4(center, 1.0);
    let nearest = clip.w - radius * length(m[3].xyz);
    if nearest <= 0.0 { return true; }
    // Bound the change in x/w and y/w over the sphere, including perspective
    // magnification off-axis. The bound can keep a meshlet, never shrink it.
    let ndc = clip.xy / clip.w;
    let pixels = vec2(length(m[0].xyz - ndc.x * m[3].xyz),
                      length(m[1].xyz - ndc.y * m[3].xyz))
        * (radius / nearest) * secondary.extent.xy;
    return max(pixels.x, pixels.y) >= secondary.options.z;
}

@compute @workgroup_size(64) fn cull_secondary(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if index >= cull.counts.x { return; }
    let b = bounds[index];
    let center = (cull.model * vec4(b.center, 1.0)).xyz;
    let radius = b.radius * cull.camera_scale.w;
    let m = transpose(cull.view_proj);
    let in_frustum = !(outside(m[3] + m[0], center, radius) || outside(m[3] - m[0], center, radius)
        || outside(m[3] + m[1], center, radius) || outside(m[3] - m[1], center, radius)
        || outside(m[3] - m[2], center, radius) || outside(m[2], center, radius));
    let kept_side = secondary.options.x == 0.0 || !outside(secondary.clip_plane, center, radius);
    // The normal cone is camera-relative, just as on the ordinary cooked path.
    var keep = in_frustum && kept_side && large_enough(m, center, radius);
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
}
