fn ground_interval(camera: vec3<f32>, ray: vec3<f32>, end: f32) -> vec2<f32> {
    return vec2(end, 0.0);
}

fn ground_near_weight(distance: f32) -> f32 {
    return 0.0;
}

fn ground_sky_limit(ray: vec3<f32>, true_sun: vec3<f32>) -> vec4<f32> {
    return vec4(0.0);
}

fn ground_light(point: vec3<f32>, ray: vec3<f32>, true_sun: vec3<f32>,
                distance: f32, transmission: f32, coarse: bool, sky_limit: vec4<f32>) -> GroundLight {
    return GroundLight(0.0, 0.0, vec3(0.0), 1.0);
}
