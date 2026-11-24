struct SoftSunFrame {
    inverse: mat4x4<f32>,
    previous: mat4x4<f32>,
    camera: vec4<f32>,
    extent: vec2<u32>,
    jitter_delta: vec2<f32>,
    enabled: u32,
    valid: u32,
    phase: u32,
    settle: u32,
    dive_cosine: f32,
    radius: f32,
    padding: vec2<u32>,
}

fn soft_within(pixel: vec2<i32>, extent: vec2<u32>) -> bool {
    return all(pixel >= vec2(0)) && all(pixel < vec2<i32>(extent));
}

fn soft_depth_matches(stored: f32, expected: f32) -> bool {
    return stored > 0.0 && expected > 0.0 &&
        abs(stored - expected) <= max(1e-6, 0.025 * expected);
}
