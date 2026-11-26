fn write_overlay_light(light: vec3<f32>, sky_lux: f32, active_overlay: bool) {
    if active_overlay {
        environment_light.sky_diffuse[3].w = sky_lux + overlay_open_sun();
    }
}
