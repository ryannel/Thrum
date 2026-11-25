struct AmbientFrame {
    inverse: mat4x4<f32>,
    previous: mat4x4<f32>,
    camera: vec4<f32>,
    extent: vec2<u32>,
    jitter_delta: vec2<f32>,
    stride: u32,
    valid: u32,
    phase: u32,
    settle: u32,
    radius: f32,
    padding0: u32,
    padding1: u32,
    padding2: u32,
}

fn ambient_extent(frame: AmbientFrame) -> vec2<u32> {
    return (frame.extent + frame.stride - 1u) / frame.stride;
}

fn ambient_in_bounds(pixel: vec2<i32>, extent: vec2<u32>) -> bool {
    return all(pixel >= vec2(0)) && all(pixel < vec2<i32>(extent));
}

fn ambient_depth_weight(stored: f32, expected: f32) -> f32 {
    if stored <= 0.0 || expected <= 0.0 { return 0.0; }
    return 1.0 - smoothstep(0.002, 0.018, abs(stored - expected) / expected);
}
