@group(0) @binding(0) var source_rungs: texture_2d_array<f32>;
@group(0) @binding(1) var caustic_sampler: sampler;
@group(0) @binding(2) var filtered_rungs: texture_storage_2d_array<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn blur(@builtin(global_invocation_id) slot: vec3<u32>) {
    let size = textureDimensions(filtered_rungs);
    if any(slot.xy >= size) { return; }
    let uv = (vec2<f32>(slot.xy) + 0.5) / vec2<f32>(size);
    // Unresolved wave slopes and the solar disc widen the projected spot with depth.
    let radius = caustic_rung_depth(slot.z) * (0.00465 + 0.25 * caustic_draw.sun.w)
        / caustic_draw.window.z;
    let weight = array(0.25, 0.5, 0.25);
    var irradiance = 0.0;
    for (var y = 0; y < 3; y++) {
        for (var x = 0; x < 3; x++) {
            let offset = vec2<f32>(f32(x - 1), f32(y - 1)) * radius;
            irradiance += weight[x] * weight[y] *
                textureSampleLevel(source_rungs, caustic_sampler, uv + offset, i32(slot.z), 0.0).r;
        }
    }
    textureStore(filtered_rungs, vec2<i32>(slot.xy), i32(slot.z), vec4(irradiance, 0.0, 0.0, 0.0));
}

@compute @workgroup_size(8, 8, 1)
fn reduce(@builtin(global_invocation_id) slot: vec3<u32>) {
    let size = textureDimensions(filtered_rungs);
    if any(slot.xy >= size) { return; }
    let uv = (vec2<f32>(slot.xy) + 0.5) / vec2<f32>(size);
    // At this mip's texel centre, the linear read averages four source texels.
    let value = textureSampleLevel(source_rungs, caustic_sampler, uv, i32(slot.z), 0.0);
    textureStore(filtered_rungs, vec2<i32>(slot.xy), i32(slot.z), value);
}
