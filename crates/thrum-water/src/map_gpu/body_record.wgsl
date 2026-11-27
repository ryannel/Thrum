struct BodyRecord {
    level: vec4<f32>,
    gains: vec4<f32>,
    layers: vec4<u32>,
    ends: array<vec4<f32>, 2>,
    joins: array<vec4<f32>, 2>,
    track: vec4<f32>, // parallel reference and axis, or bounded layer, period limit and mode
}
