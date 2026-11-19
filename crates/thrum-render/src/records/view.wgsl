struct ViewRecord {
    view_proj: mat4x4<f32>,
    previous_view_proj: mat4x4<f32>,
    camera_xyz_time: vec4<f32>,
    pre_exposure_inv_exposure_jitter: vec4<f32>,
    unjittered_view_proj: mat4x4<f32>,
    mip_bias: f32,
    jitter_phase: u32,
    padding: vec2<u32>,
}
