@group(0) @binding(6) var<uniform> overlay_sun: DirectionalLight;

fn overlay_meter_sample(direction: vec3<f32>, encoded: vec4<f32>) -> vec4<f32> {
    let clear = dot(overlay_clear(encoded), REC709_LUMA);
    return vec4(overlay_radiance(direction, encoded, aerial_sampler), clear);
}

fn write_haze_scale(light: vec4<f32>) {
    let direct = dot(overlay_sun.irradiance.rgb, REC709_LUMA) * max(overlay_sun.direction_to_light.y, 0.0);
    let clear = light.w + direct;
    let shaded = dot(light.rgb, REC709_LUMA) + direct * overlay_sun_factor();
    var scale = 1.0;
    if clear > 1e-8 && abs(shaded - clear) > clear * 1e-6 {
        scale = clamp(shaded / clear, 0.0, 1.0);
    }
    // The RGB lanes are the sky SH coefficients. The second coefficient's
    // spare W lane carries the aerial scale in this producer only.
    environment_light.sky_diffuse[1].w = scale;
}
