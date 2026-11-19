struct Atmosphere {
    inverse_view_projection: mat4x4<f32>,
    camera_altitude: vec4<f32>,
    sun_lux: vec4<f32>,
    colour_pre_exposure: vec4<f32>,
    sky_mapping: vec4<f32>, // Horizon mu/elevation, sky/ground angular spans.
}
