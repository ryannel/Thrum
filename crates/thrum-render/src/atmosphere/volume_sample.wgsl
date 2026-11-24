fn aerial_sample(distance_metres: f32, uv: vec2<f32>) -> AerialSample {
    let distance = min(32000.0, distance_metres);
    let layers = textureNumLayers(aerial_scatter) - 2u;
    let coordinate = volume_grid_coordinate(distance, layers);
    let low = min(layers - 1u, u32(floor(coordinate)));
    let high = low + 1u;
    let first_distance = volume_grid_distance(low, layers);
    let second_distance = volume_grid_distance(high, layers);
    let blend = clamp((distance - first_distance) / max(0.001, second_distance - first_distance), 0.0, 1.0);
    let scatter = mix(
        textureSampleLevel(aerial_scatter, aerial_sampler, uv, i32(low), 0.0),
        textureSampleLevel(aerial_scatter, aerial_sampler, uv, i32(high), 0.0),
        blend);
    let transmission = mix(
        textureSampleLevel(aerial_transmittance, aerial_sampler, uv, i32(low), 0.0),
        textureSampleLevel(aerial_transmittance, aerial_sampler, uv, i32(high), 0.0),
        blend);
    return AerialSample(scatter, transmission.rgb, transmission.a / NIGHT_LUT_GAIN + scatter.a);
}
