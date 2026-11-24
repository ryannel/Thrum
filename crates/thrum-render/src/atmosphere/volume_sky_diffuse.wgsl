@group(0) @binding(7) var volume_transmission: texture_2d_array<f32>;

fn terminal_diffuse(uv: vec2<f32>) -> f32 {
    let layer = i32(textureNumLayers(volume_transmission)) - 1;
    return textureSampleLevel(volume_transmission, lut_sampler, uv, layer, 0.0).a;
}
