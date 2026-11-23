@group(1) @binding(4) var aerial_scatter: texture_2d_array<f32>;
@group(1) @binding(5) var aerial_transmittance: texture_2d_array<f32>;
@group(1) @binding(6) var aerial_sampler: sampler;
@group(1) @binding(7) var<uniform> aerial_frame: Atmosphere;

// Cumulative in-scatter (RGB; alpha is the Sun's single-scattered luminance) and
// transmission from the camera to `distance` along this pixel's froxel column,
// still at the volume's SKY_LUT_GAIN storage scale.
struct AerialSample {
    scatter: vec4<f32>,
    transmission: vec3<f32>,
    night: f32,
}
