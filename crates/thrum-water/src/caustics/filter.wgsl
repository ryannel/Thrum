@group(0) @binding(0) var source_rungs: texture_2d_array<f32>;
@group(0) @binding(1) var caustic_sampler: sampler;
@group(0) @binding(2) var filtered_rungs: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(3) var<storage, read> rung_mean: array<f32>;

fn focal_blur(slot: vec3<u32>, size: vec2<u32>, axis: vec2<f32>) -> f32 {
    let uv = (vec2<f32>(slot.xy) + 0.5) / vec2<f32>(size);
    let centre = textureSampleLevel(source_rungs, caustic_sampler, uv, i32(slot.z), 0.0).r;
    // The normal spread of unresolved waves and the Sun's disc set one world
    // scale. Integrate within three deviations, pairing neighbouring texels
    // with linear reads. The support is contiguous even on deep rungs.
    let depth = caustic_rung_depth(slot.z);
    // The fine shallow grids resolve more of the projected convergence.
    // Blend into the broader tail on the coarser rungs.
    let tail_share = mix(0.04, 0.25, smoothstep(2.8, 5.7, depth));
    let spread = depth * (0.00465 + tail_share * caustic_draw.sun.w);
    let sigma = spread * f32(size.x) / (sqrt(2.0) * caustic_draw.window.z);
    if sigma < 0.26 { return centre; }
    let pairs = min(12u, u32(ceil(1.5 * sigma)));
    let ratio = exp(-0.5 / (sigma * sigma));
    let ratio_sq = ratio * ratio;
    var first = ratio;
    var successive = ratio * ratio_sq;
    var difference = 0.0;
    var weights = 1.0;
    for (var pair = 0u; pair < pairs; pair++) {
        let second = first * successive;
        let weight = first + second;
        let distance = f32(2u * pair + 1u) + second / max(weight, 1e-20);
        let delta = axis * distance / vec2<f32>(size);
        let before = textureSampleLevel(source_rungs, caustic_sampler, uv - delta, i32(slot.z), 0.0).r;
        let after = textureSampleLevel(source_rungs, caustic_sampler, uv + delta, i32(slot.z), 0.0).r;
        difference += ((before - centre) + (after - centre)) * weight;
        weights += 2.0 * weight;
        successive *= ratio_sq;
        first = second * successive;
        successive *= ratio_sq;
    }
    // A uniform input stays bit exact between the two half-float stores.
    return centre + difference / weights;
}

@compute @workgroup_size(8, 8, 1)
fn blur_horizontal(@builtin(global_invocation_id) slot: vec3<u32>) {
    let size = textureDimensions(filtered_rungs);
    if any(slot.xy >= size) { return; }
    let irradiance = focal_blur(slot, size, vec2(1.0, 0.0));
    textureStore(filtered_rungs, vec2<i32>(slot.xy), i32(slot.z), vec4(irradiance, 0.0, 0.0, 0.0));
}

@compute @workgroup_size(8, 8, 1)
fn blur_vertical(@builtin(global_invocation_id) slot: vec3<u32>) {
    let size = textureDimensions(filtered_rungs);
    if any(slot.xy >= size) { return; }
    let irradiance = focal_blur(slot, size, vec2(0.0, 1.0));
    let mean = rung_mean[slot.z];
    let balanced = select(1.0, irradiance / max(mean, 0.001), mean > 0.001);
    textureStore(filtered_rungs, vec2<i32>(slot.xy), i32(slot.z), vec4(balanced, 0.0, 0.0, 0.0));
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
