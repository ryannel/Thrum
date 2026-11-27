fn aerial_uv(screen_position: vec4<f32>) -> vec2<f32> {
    return clamp(screen_position.xy * view_aux.viewport_inv_viewport.zw, vec2(0.0), vec2(1.0));
}
// Decode the stored solar term and apply the registered haze scale once.
fn aerial_radiance(scatter: vec3<f32>) -> vec3<f32> {
    return (scatter * (aerial_frame.sun_lux.w / SKY_LUT_GAIN) * aerial_frame.colour_pre_exposure.rgb)
        * chart_haze_scale();
}

fn composite_aerial(sample: AerialSample, surface: vec3<f32>, solar_scatter: vec3<f32>) -> vec3<f32> {
    return sample.transmission * surface + aerial_radiance(solar_scatter) + vec3(sample.night);
}
