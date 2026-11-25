@group(0) @binding(5) var sky_overlay: texture_cube<f32>;

fn overlay_clear(encoded: vec4<f32>) -> vec3<f32> {
    return decode_sky_light(encoded, atmosphere.sun_lux.w * atmosphere.colour_pre_exposure.rgb);
}

fn overlay_radiance(direction: vec3<f32>, encoded: vec4<f32>, overlay_sampler: sampler) -> vec3<f32> {
    let layer = textureSampleLevel(sky_overlay, overlay_sampler, direction, 0.0);
    return overlay_clear(encoded) * layer.a + layer.rgb;
}
