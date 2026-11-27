@group(0) @binding(0) var source_rungs: texture_2d_array<f32>;
@group(0) @binding(1) var caustic_sampler: sampler;
@group(0) @binding(2) var filtered_rungs: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(3) var<storage, read> rung_mean: array<f32>;

// Equal-area samples over the focal spot, with the former axial variance.
// A disk avoids separate images of narrow peaks at regularly spaced taps.
const FOCAL_SPOT = array<vec2<f32>, 16>(
    vec2(0.25000000, 0.00000000),
    vec2(-0.31929009, 0.29249588),
    vec2(0.04887247, -0.55687654),
    vec2(0.40244448, 0.52491756),
    vec2(-0.73853511, -0.13063646),
    vec2(0.69960493, -0.44503139),
    vec2(-0.23400416, 0.87048380),
    vec2(-0.44627131, -0.85926825),
    vec2(0.96823023, 0.35359613),
    vec2(-1.00728222, 0.41579146),
    vec2(0.48557659, -1.03764897),
    vec2(0.35882875, 1.14400259),
    vec2(-1.08151401, -0.62675948),
    vec2(1.26873905, -0.27892872),
    vec2(-0.77429169, 1.10135025),
    vec2(-0.17887931, -1.38039929),
);

@compute @workgroup_size(8, 8, 1)
fn blur(@builtin(global_invocation_id) slot: vec3<u32>) {
    let size = textureDimensions(filtered_rungs);
    if any(slot.xy >= size) { return; }
    let uv = (vec2<f32>(slot.xy) + 0.5) / vec2<f32>(size);
    // Unresolved wave slopes and the solar disc widen the projected spot with depth.
    let radius = caustic_rung_depth(slot.z) * (0.00465 + 0.25 * caustic_draw.sun.w)
        / caustic_draw.window.z;
    var irradiance = 0.0;
    for (var tap = 0u; tap < 16u; tap++) {
        irradiance += textureSampleLevel(source_rungs, caustic_sampler,
            uv + FOCAL_SPOT[tap] * radius, i32(slot.z), 0.0).r;
    }
    irradiance *= 0.0625;
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
