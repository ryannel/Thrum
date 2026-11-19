fn sun_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    let direction = light.direction_to_light.xyz;
    if dot(receiver.normal, direction) <= 0.0 { return 1.0; }
    return select(1.0, 0.0, shadow_ray_blocked(shadow_ray_origin(receiver.position, receiver.normal), direction, 1e30));
}
