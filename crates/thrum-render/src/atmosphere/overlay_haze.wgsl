fn write_overlay_light(light: vec3<f32>, sky_lux: f32, active_overlay: bool) {
    // The RGB lanes remain the sky SH coefficients. The auxiliary lanes
    // carry the local aerial scale and the distributed display meter inputs.
    environment_light.sky_diffuse[1].w = 1.0;
    if !active_overlay { return; }
    let direct = overlay_open_sun();
    let clear = sky_lux + direct;
    let shaded = dot(light, REC709_LUMA) + direct * overlay_sun_factor();
    var scale = 1.0;
    if clear > 1e-8 && abs(shaded - clear) > clear * 1e-6 {
        scale = clamp(shaded / clear, 0.0, 1.0);
    }
    environment_light.sky_diffuse[1].w = scale;
    environment_light.sky_diffuse[3].w = clear;
}
