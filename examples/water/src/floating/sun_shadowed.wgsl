fn floating_sun_factor(point: vec3<f32>, normal: vec3<f32>, clip: vec4<f32>) -> f32 {
    let path = sun_path(point, light.direction_to_light.xyz);
    let receiver = ShadowReceiver(point, dpdx(point), dpdy(point), normal);
    var input: VertexOutput;
    input.position = clip;
    if !path.submerged {
        return sun_visibility(input, receiver) * floating_cloud_factor(point);
    }
    let visibility = point_sun_visibility(path.exit, sun_path_shadow_sampler);
    return path.factor * visibility * floating_cloud_factor(point);
}
