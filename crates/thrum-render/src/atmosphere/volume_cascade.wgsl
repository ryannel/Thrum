@group(1) @binding(0) var<uniform> volume_shadows: ShadowCascades;
@group(1) @binding(1) var volume_shadow_maps: texture_depth_2d_array;
@group(1) @binding(2) var volume_shadow_filter: sampler_comparison;

// The cascades use reverse Z and an orthographic light projection. Compared
// with receivers on mesh surfaces, cells need only the constant depth bias.
// The linear comparison filter softens one texel at the shadow boundary.
fn volume_cascade(index: u32, point: vec3<f32>) -> f32 {
    let clip = volume_shadows.light_view_proj[index] * vec4(point, 1.0);
    let p = clip.xyz / clip.w;
    let uv = p.xy * vec2(0.5, -0.5) + vec2(0.5);
    if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) || p.z < 0.0 || p.z > 1.0 {
        return 1.0;
    }
    let bias = select(volume_shadows.bias.y, volume_shadows.bias.x, index == 0u);
    return textureSampleCompareLevel(volume_shadow_maps, volume_shadow_filter, uv,
        i32(index), p.z + bias);
}

fn volume_terrain_visibility(point: vec3<f32>, depth: f32) -> f32 {
    let split = volume_shadows.split_overlap.x;
    let overlap = volume_shadows.split_overlap.y;
    let reach = volume_shadows.split_overlap.z;
    let fade_start = volume_shadows.split_overlap.w;
    let weight = select(0.0, smoothstep(fade_start, reach, depth), reach > fade_start);
    if weight >= 1.0 { return 1.0; }
    if depth <= split - overlap {
        // The near map may clip tall or far upstream casters into the far map.
        return min(volume_cascade(0u, point), volume_cascade(1u, point));
    }
    let far = volume_cascade(1u, point);
    if depth >= split + overlap { return mix(far, 1.0, weight); }
    let near = min(volume_cascade(0u, point), far);
    return mix(near, far, smoothstep(split - overlap, split + overlap, depth));
}
