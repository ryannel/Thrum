// A stack consists of power-of-two cubes packed consecutively along Z.
// Trilinear filtering may use the mip on either side of a fractional level.
// Keep the lookup half a texel of the coarser level inside the chosen slab.
fn placed_slab(
    stack: texture_3d<f32>, volume_sampler: sampler, slab: u32,
    local: vec3<f32>, requested_level: f32,
) -> f32 {
    if any(local < vec3(0.0)) || any(local > vec3(1.0)) {
        return 0.0;
    }
    let base = textureDimensions(stack, 0);
    let slab_count = f32(base.z / base.x);
    let last_isolated = min(u32(firstLeadingBit(base.x)), textureNumLevels(stack) - 1u);
    let level = clamp(requested_level, 0.0, f32(last_isolated));
    let coarser = textureDimensions(stack, u32(ceil(level)));
    let inset = 0.5 / f32(coarser.x);
    let z = clamp(local.z, inset, 1.0 - inset);
    return textureSampleLevel(stack, volume_sampler, vec3(local.xy, (f32(slab) + z) / slab_count), level).r;
}

fn placed_form_weights(volume: f32) -> vec2<f32> {
    let weight = clamp(volume, 0.0, 1.0);
    return vec2(1.0 - weight, weight);
}
