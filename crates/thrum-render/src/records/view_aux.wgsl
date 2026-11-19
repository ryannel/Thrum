struct ViewAuxRecord {
    inverse_view_proj: mat4x4<f32>,
    viewport_inv_viewport: vec4<f32>,
    near_far: vec4<f32>,
    time: vec4<f32>,
    flags: vec4<u32>,
    subrectangle: vec4<f32>,
}
