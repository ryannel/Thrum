fn environment_far_sky(direction: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let point = clamp(uv, vec2(0.0), vec2(1.0));
    let layer = i32(textureNumLayers(aerial_scatter)) - 1;
    let terminal = textureSampleLevel(aerial_scatter, aerial_sampler, point, layer, 0.0);
    let diffuse = textureSampleLevel(aerial_transmittance, aerial_sampler, point, layer, 0.0).a;
    return terminal.a * environment_clear_sky_radiance(direction) + aerial_radiance(terminal.rgb) + vec3(diffuse);
}
