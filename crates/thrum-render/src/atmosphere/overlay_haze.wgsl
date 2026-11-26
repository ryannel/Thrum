@group(0) @binding(6) var<uniform> overlay_sun: DirectionalLight;

fn write_overlay_light(light: vec3<f32>, coverage: vec2<f32>, active_overlay: bool) {
    // The RGB lanes remain the sky SH coefficients. The auxiliary lanes
    // carry the local aerial scale and the distributed display meter inputs.
    environment_light.sky_diffuse[1].w = 1.0;
    if !active_overlay { return; }
    let direct = dot(overlay_sun.irradiance.rgb, REC709_LUMA) * max(overlay_sun.direction_to_light.y, 0.0);
    let clear = coverage.x + direct;
    let shaded = dot(light, REC709_LUMA) + direct * overlay_sun_factor();
    var scale = 1.0;
    if clear > 1e-8 && abs(shaded - clear) > clear * 1e-6 {
        scale = clamp(shaded / clear, 0.0, 1.0);
    }
    environment_light.sky_diffuse[1].w = scale;
    environment_light.sky_diffuse[3].w = clear;
    environment_light.sky_diffuse[4].w = clamp(coverage.y / PI, 0.0, 1.0);
}
