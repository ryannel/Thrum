@vertex fn triangle(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    let corner = vec2f(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4f(corner * 2.0 - 1.0, 1.0, 1.0);
}

@fragment fn deposit() -> @location(0) vec4f {
    return vec4f(1.0);
}

@group(0) @binding(0) var field: texture_2d_array<f32>;

struct Forward {
    @location(0) colour: vec4f,
    @location(1) motion: vec2f,
    @location(2) reactive: f32,
}

@fragment fn show_field(@builtin(position) pixel: vec4f) -> Forward {
    let width = i32(textureDimensions(field).x);
    let x = i32(pixel.x);
    let light = textureLoad(field, vec2i(x % width, i32(pixel.y)), x / width, 0).r;
    return Forward(vec4f(light, 0.0, 0.0, 1.0), vec2f(0.0), 0.0);
}
