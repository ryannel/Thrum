fn aerial_colour(colour: vec3<f32>, world_position: vec3<f32>, screen_position: vec4<f32>) -> vec3<f32> {
    let ray = secondary_air_ray(world_position, screen_position);
    let sample = aerial_sample(ray.distance, aerial_uv(screen_position));
    return composite_aerial(sample, colour, sample.scatter.rgb);
}
