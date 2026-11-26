struct Parameters { level_dt_reset: vec4<f32>, gains: vec4<f32>, lengths: vec4<f32>, cache: array<vec4<f32>,8>, long_covariance:array<vec4<f32>,8>, scatter:vec4<f32>, heights:vec4<f32>, carriage:vec4<f32>, }
@group(3) @binding(0) var<uniform> water: Parameters;
@group(3) @binding(1) var cascades:texture_2d_array<f32>;
@group(3) @binding(2) var long_field:texture_2d_array<f32>;
@group(3) @binding(3) var periodic:sampler;
@group(3) @binding(4) var bounded:sampler;
