struct SecondaryViewRecord {
    view_proj: mat4x4<f32>,
    inverse_view_proj: mat4x4<f32>,
    unjittered_view_proj: mat4x4<f32>,
    inverse_unjittered_view_proj: mat4x4<f32>,
    camera: vec4<f32>,
    extent: vec4<f32>,
    clip_plane: vec4<f32>,
    options: vec4<f32>,
}
