struct QueryPoint {
    position: vec2<f32>, radius: f32, dt: f32,
    level_current_bed: vec4<f32>,
    slope: vec2<f32>, body: u32, flags: u32,
    gains: vec4<f32>,
    dent: vec4<f32>,
    surf_stage: vec4<f32>, // stage and surf cut records
    fine_bounds: vec4<i32>,
    coarse_grid: vec4<i32>,
    carry: vec4<f32>, // cascade and wake time offsets
    track: vec4<f32>,
    current_gradient: vec4<f32>, // current derivatives with respect to x, then z
    track_gradient: vec4<f32>,
