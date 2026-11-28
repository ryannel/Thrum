struct ShadowCascades {
    light_view_proj: array<mat4x4<f32>, SUN_CASCADE_COUNT>,
    // Receiver radius, texel metres, inverse depth span, minimum receiver bias.
    cascade_metrics: array<vec4<f32>, SUN_CASCADE_COUNT>,
    // Cascade count, distance reach, fade start, and first extra outline layer.
    profile: vec4<f32>,
    eye: vec4<f32>,
    height_support: vec4<f32>,
    height_view_proj: mat4x4<f32>,
    // Depth = x*height + y; uv per metre; texel metres. Zero x disables it.
    horizon: vec4<f32>,
}

struct ShadowChoice {
    layer: u32,
    blend: f32,
    fade: f32,
}

fn sun_cascade_choice(profile: vec4<f32>, first_radius: f32, distance: f32) -> ShadowChoice {
    let count = u32(profile.x);
    if count == 0u || distance >= profile.y {
        return ShadowChoice(count, 0.0, 1.0);
    }
    var index = 0u;
    var radius = first_radius;
    while distance > radius && index + 1u < count {
        index += 1u;
        radius *= 2.0;
    }
    if index + 1u < count {
        return ShadowChoice(index, smoothstep(0.75 * radius, radius, distance), 0.0);
    }
    return ShadowChoice(index, 0.0, smoothstep(profile.z, profile.y, distance));
}
