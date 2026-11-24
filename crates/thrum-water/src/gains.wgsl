fn limit_gains(gains: vec4<f32>, depth: f32, heights: vec4<f32>, swash_ratio: f32, margin: f32) -> vec4<f32> {
    let supported_height = length(heights * gains);
    let swash_depth = min(swash_ratio * supported_height, margin);
    let cap = 0.6 * max(0.0, depth + swash_depth);
    return select(gains, min(gains, cap / max(heights, vec4(1e-30))), heights > vec4(0.0));
}

// Significant height is four times RMS height. This is the same lost
// variance, in square metres, that the CPU map reports for each layer.
fn lost_wave_power(original: vec4<f32>, limited: vec4<f32>, heights: vec4<f32>) -> f32 {
    let before = original * heights;
    let after = limited * heights;
    return max(0.0, dot(before, before) - dot(after, after)) * 0.0625;
}

// The resolved short waves fade through the overlap. The pixel shading
// still resolves their normals independently of displaced geometry.
fn swash_short_weight(depth: f32, swash: f32) -> f32 {
    if swash <= 0.0 { return 1.0; }
    return smoothstep(0.85, 1.3, depth / swash);
}
