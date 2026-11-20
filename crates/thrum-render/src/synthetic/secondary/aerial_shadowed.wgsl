fn aerial_colour(colour: vec3<f32>, world_position: vec3<f32>, screen_position: vec4<f32>) -> vec3<f32> {
    let ray = secondary_air_ray(world_position, screen_position);
    let uv = aerial_uv(screen_position);
    let end = aerial_sample(ray.distance, uv);
    var scatter = vec3(0.0);
    var previous = vec4(0.0);
    var previous_distance = 0.0;
    for (var i = 1u; i <= AIR_INTERVALS; i++) {
        let fraction = f32(i) / f32(AIR_INTERVALS);
        let interval_end = ray.distance * fraction * fraction;
        let current = select(aerial_sample(interval_end, uv).scatter, end.scatter, i == AIR_INTERVALS);
        let delta = current - previous;
        let luminance = dot(delta.rgb, REC709_LUMA);
        let single = clamp(delta.a / max(luminance, 1e-20), 0.0, 1.0);
        let middle = ray.origin + ray.direction * (0.5 * (previous_distance + interval_end));
        let visibility = air_sun_visibility(middle);
        scatter += delta.rgb * (1.0 - single * (1.0 - visibility));
        previous = current;
        previous_distance = interval_end;
    }
    return composite_aerial(end, colour, scatter);
}
