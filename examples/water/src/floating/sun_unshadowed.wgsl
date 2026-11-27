fn floating_sun_factor(point: vec3<f32>, _normal: vec3<f32>, _clip: vec4<f32>) -> f32 {
    return sun_path(point, light.direction_to_light.xyz).factor * floating_cloud_factor(point);
}
