    return f;
}
struct SurfaceRow {
    level: f32,
    previous_level: f32,
    gains: vec4<f32>,
    bed: f32,
    owned: bool,
    swash: f32,
    loss: f32,
    loss_without_long: f32,
    current: vec2<f32>,
    track: vec4<f32>,
}
struct VertexOut {
    @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32>,
