struct ReconstructionParams {
    inverse: mat4x4<f32>,
    previous_view: mat4x4<f32>,
    view: mat4x4<f32>,
    state: vec4<f32>, // valid, reserved, drawn width, drawn height
    jitter: vec4<f32>,
    previous_extent: vec4<f32>,
}
