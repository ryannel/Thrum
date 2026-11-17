struct CloudParams {
    weather:array<vec4<f32>,4>, physical:vec4<f32>, erosion:vec4<f32>,
    drift:vec4<f32>, heights:vec4<f32>, settings:vec4<u32>, temporal:vec4<u32>, noise:vec4<u32>,
    inverse_raster:mat4x4<f32>, previous_raster:mat4x4<f32>,
}
