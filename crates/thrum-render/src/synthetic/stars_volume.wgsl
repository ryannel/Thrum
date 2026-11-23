@group(0) @binding(7) var air_terminal: texture_2d_array<f32>;

@fragment
fn fragment_view_volume(input: VertexOut) -> StarOutput {
    if textureLoad(depth, vec2<i32>(input.position.xy), 0) > 0.0 { discard; }
    let offset = input.position.xy - input.centre;
    let weight = max(normal_cdf((offset + PIXEL_HALF_WIDTH) / input.sigma) - normal_cdf((offset - PIXEL_HALF_WIDTH) / input.sigma), vec2(0.0));
    if weight.x * weight.y <= 0.0 { discard; }
    let uv = input.position.xy / view.extent_and_tan_half_fov.xy;
    let transmission = textureSampleLevel(air_terminal, lut_sampler, uv,
        i32(textureNumLayers(air_terminal)) - 1, 0.0).a;
    return StarOutput(vec4(input.scale * weight.x * weight.y * transmission, 0.0), input.motion, view.jitter.z);
}
