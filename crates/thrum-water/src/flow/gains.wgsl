fn flow_swatch(wind: vec4<f32>, flow: vec2<f32>, ratio: f32, margin: f32) -> f32 {
    let flow_height = flow * FLOW_HEIGHTS;
    return min(ratio * sqrt(dot(wind, wind) + dot(flow_height, flow_height)), margin);
}
fn limit_flow(gains: vec2<f32>, depth: f32, swash: f32) -> vec2<f32> {
    return min(gains, 0.6 * max(0.0, depth + swash) / FLOW_HEIGHTS);
}
fn limit_wind_with_flow(gains: vec4<f32>, heights: vec4<f32>, depth: f32, swash: f32) -> vec4<f32> {
    return select(gains, min(gains, 0.6 * max(0.0, depth + swash) / max(heights, vec4(1e-30))),
        heights > vec4(0.0));
}
