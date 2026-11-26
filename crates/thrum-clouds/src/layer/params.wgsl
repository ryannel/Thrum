struct CloudParams {
    weather:array<vec4<f32>,4>, physical:vec4<f32>, erosion:vec4<f32>, scattering:vec4<f32>,
    drift:vec4<f32>, heights:vec4<f32>, light_samples:u32, sun_angular_radius:f32, render_extent:vec2<u32>,
    temporal:vec4<u32>, noise:vec4<f32>, bank_noise:vec4<f32>, density_profile:vec4<f32>,
    active_extent:vec2<u32>, sample_cap:u32, field_overlay:u32,
    inverse_raster:mat4x4<f32>, previous_raster:mat4x4<f32>,
}
