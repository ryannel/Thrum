fn decode_overlay_reflection(encoded: vec4<f32>, solar: vec3<f32>, cloudy: bool) -> vec3<f32> {
    return select(decode_sky_light(encoded, solar), encoded.rgb * OVERLAY_REFLECTION_GAIN, cloudy);
}

fn sample_overlay_reflection(encoded: vec4<f32>, direction: vec3<f32>, level: f32, solar: vec3<f32>) -> vec3<f32> {
    if chart_lighting.mode == 0u {
        return decode_overlay_reflection(encoded, solar, environment_light.sky_diffuse[2].w != 0.0);
    }
    let lower = u32(floor(level));
    let upper = min(lower + 1u, textureNumLevels(sky_reflection) - 1u);
    let lower_cloudy = reflection_tags.tags[lower / 4u][lower % 4u] != 0u;
    let upper_cloudy = reflection_tags.tags[upper / 4u][upper % 4u] != 0u;
    if lower_cloudy == upper_cloudy || fract(level) == 0.0 {
        return decode_overlay_reflection(encoded, solar, lower_cloudy);
    }
    let a = textureSampleLevel(sky_reflection, sky_reflection_sampler, direction, f32(lower));
    let b = textureSampleLevel(sky_reflection, sky_reflection_sampler, direction, f32(upper));
    return mix(decode_overlay_reflection(a, solar, lower_cloudy),
        decode_overlay_reflection(b, solar, upper_cloudy), fract(level));
}

fn chart_haze_scale() -> f32 {
    return select(chart_lighting.haze, environment_light.sky_diffuse[1].w, chart_lighting.mode == 0u);
}
