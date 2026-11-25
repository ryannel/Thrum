struct ViewRecord {
    view_proj: mat4x4<f32>, previous_view_proj: mat4x4<f32>,
    camera_xyz_time: vec4<f32>, pre_exposure_inv_exposure_jitter: vec4<f32>,
    unjittered_view_proj: mat4x4<f32>, mip_bias: f32, jitter_phase: u32, padding: vec2<u32>,
}
struct ViewAuxRecord {
    inverse_view_proj: mat4x4<f32>, viewport_inv_viewport: vec4<f32>,
    near_far: vec4<f32>, time: vec4<f32>, flags: vec4<u32>, subrectangle: vec4<f32>,
}
@group(0) @binding(0) var<uniform> view: ViewRecord;
@group(0) @binding(1) var<uniform> view_aux: ViewAuxRecord;
