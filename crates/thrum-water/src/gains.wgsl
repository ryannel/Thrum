fn limit_gains(gains: vec4<f32>, depth: f32, heights: vec4<f32>, swash_ratio: f32, margin: f32) -> vec4<f32> {
    let supported_height = length(heights * gains);
    let swash_depth = min(swash_ratio * supported_height, margin);
    let cap = 0.6 * max(0.0, depth + swash_depth);
    return select(gains, min(gains, cap / max(heights, vec4(1e-30))), heights > vec4(0.0));
}
