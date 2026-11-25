@group(0) @binding(1) var sky: texture_2d<f32>;
@group(0) @binding(2) var lut_sampler: sampler;
@group(0) @binding(3) var<storage,read_write> environment_light: EnvironmentLight;
var<workgroup> irradiance: array<vec4<f32>,64>;
var<workgroup> moments: array<array<vec4<f32>,9>,64>;
