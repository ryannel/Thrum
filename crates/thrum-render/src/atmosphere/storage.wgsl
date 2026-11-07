// Shared RGB scale at Sky, cube and aerial storage boundaries.
// Transport remains per unit Sun; consumers decode before lighting/exposure.
const SKY_LUT_GAIN: f32 = 1024.0;

// Night alpha is absolute luminance and is independent of Sun intensity.
const NIGHT_LUT_GAIN: f32 = 65536.0;
fn decode_sky_light(encoded: vec4<f32>, solar: vec3<f32>) -> vec3<f32> {
    return encoded.rgb / SKY_LUT_GAIN * solar + vec3(encoded.a / NIGHT_LUT_GAIN);
}
