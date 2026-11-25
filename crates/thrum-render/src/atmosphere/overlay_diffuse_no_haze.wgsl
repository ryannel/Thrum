fn overlay_meter_sample(direction: vec3<f32>, encoded: vec4<f32>) -> vec4<f32> {
    return vec4(overlay_radiance(direction, encoded, aerial_sampler), 0.0);
}

fn write_haze_scale(irradiance: vec4<f32>) {}
