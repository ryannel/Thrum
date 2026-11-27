fn floating_sun_factor(point: vec3<f32>, normal: vec3<f32>) -> f32 {
    let path = sun_path(point, light.direction_to_light.xyz);
    var visibility = 1.0;
    if path.weight >= 1.0 {
        visibility = point_sun_visibility(path.exit);
    } else {
        let ordinary = point_sun_visibility(point + normal * 0.025);
        if path.weight > 0.0 {
            visibility = mix(ordinary, point_sun_visibility(path.exit), path.weight);
        } else {
            visibility = ordinary;
        }
    }
    return path.factor * visibility * floating_cloud_factor(point);
}
