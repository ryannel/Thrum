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
    let bias = volume_shadows.cascade_metrics[index].w;
    return textureSampleCompareLevel(volume_shadow_maps, volume_shadow_filter, uv,
        i32(index), p.z + bias);
}

fn volume_terrain_visibility(point: vec3<f32>, distance: f32) -> f32 {
    let choice = sun_cascade_choice(volume_shadows.profile, volume_shadows.cascade_metrics[0].x, distance);
    if choice.fade >= 1.0 { return 1.0; }
    var value = volume_cascade(choice.layer, point);
    if choice.blend > 0.0 {
        value = mix(value, volume_cascade(choice.layer + 1u, point), choice.blend);
    }
    return mix(value, 1.0, choice.fade);
}

fn volume_cascade_fade(distance: f32) -> f32 {
    return sun_cascade_choice(volume_shadows.profile, volume_shadows.cascade_metrics[0].x, distance).fade;
}
