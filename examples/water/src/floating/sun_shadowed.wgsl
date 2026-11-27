fn floating_sun_factor(point: vec3<f32>, normal: vec3<f32>, clip: vec4<f32>) -> f32 {
    let path = sun_path(point, light.direction_to_light.xyz);
    let receiver = ShadowReceiver(point, dpdx(point), dpdy(point), normal);
    let exit = path.exit;
    let entrance = ShadowReceiver(exit, dpdx(exit), dpdy(exit), vec3(0.0, 1.0, 0.0));
    var input: VertexOutput;
    input.position = clip;
    if path.weight <= 0.0 {
        return sun_visibility(input, receiver) * floating_cloud_factor(point);
    }
    let projected = view.view_proj * vec4(exit, 1.0);
    input.position.z = projected.z / max(projected.w, 1e-4);
    let refracted = sun_visibility(input, entrance);
    var visibility = refracted;
    if path.weight < 1.0 {
        input.position = clip;
        visibility = mix(sun_visibility(input, receiver), refracted, path.weight);
    }
    return path.factor * visibility * floating_cloud_factor(point);
}
