@group(0) @binding(0) var previous: texture_2d<f32>;
@group(0) @binding(1) var output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var next: texture_storage_2d<rgba16float, write>;
@compute @workgroup_size(1) fn toggle() {
    let bit = u32(textureLoad(previous, vec2u(0), 0).x);
    let value = select(vec4f(0.0), vec4f(1.0), bit == 0u);
    textureStore(output, vec2u(0), value);
    textureStore(next, vec2u(0), value);
}
