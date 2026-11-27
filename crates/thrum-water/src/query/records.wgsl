}
struct Settings {
    origin: vec2<f32>, count: u32, components: u32,
    lengths_chop: vec4<f32>, heights: vec4<f32>, swash_margin: vec4<f32>,
    coarse_jobs: vec4<u32>,
}
struct CascadeSample {
    height: f32,
    slope: array<f32, 2>,
    differential: array<f32, 4>,
    velocity: array<f32, 3>,
}
struct Answer {
    height_slope_rate: vec4<f32>,
    velocity_depth: vec4<f32>,
    layers: vec4<f32>,
    current_bed: vec4<f32>,
    body_flags: vec4<u32>,
    cascades: array<CascadeSample, 3>,
    cut_fine_nodes: u32,
    cut_coarse_nodes: u32,
    advection_rate: array<f32, 3>,
    alignment: f32,
    wake: vec4<f32>,
