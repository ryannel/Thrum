struct AerialFrame {
    inverse_view_projection: mat4x4<f32>,
    camera_altitude: vec4<f32>,
    sun_lux: vec4<f32>,
    colour_pre_exposure: vec4<f32>,
    sky_mapping: vec4<f32>, // Mirrors Atmosphere; screen extent belongs to ViewAux.
}
@group(1) @binding(4) var aerial_scatter: texture_2d_array<f32>;
@group(1) @binding(5) var aerial_transmittance: texture_2d_array<f32>;
@group(1) @binding(6) var aerial_sampler: sampler;
@group(1) @binding(7) var<uniform> aerial_frame: AerialFrame;

// Cumulative in-scatter (RGB; alpha is the Sun's single-scattered luminance) and
// transmission from the camera to `distance` along this pixel's froxel column,
// still at the volume's SKY_LUT_GAIN storage scale.
struct AerialSample {
    scatter: vec4<f32>,
    transmission: vec3<f32>,
}
fn aerial_sample(distance_metres: f32, uv: vec2<f32>) -> AerialSample {
    let distance = min(32000.0, distance_metres);
    let coordinate = sqrt(distance / 32000.0) * 32.0;
    let low = min(31u, u32(floor(coordinate)));
    let high = low + 1u;
    let first_distance = 32000.0 * pow(f32(low) / 32.0, 2.0);
    let second_distance = 32000.0 * pow(f32(high) / 32.0, 2.0);
    let blend = clamp((distance - first_distance) / max(0.001, second_distance - first_distance), 0.0, 1.0);
    let scatter = mix(
        textureSampleLevel(aerial_scatter, aerial_sampler, uv, i32(low), 0.0),
        textureSampleLevel(aerial_scatter, aerial_sampler, uv, i32(high), 0.0),
        blend);
    let transmission = mix(
        textureSampleLevel(aerial_transmittance, aerial_sampler, uv, i32(low), 0.0).rgb,
        textureSampleLevel(aerial_transmittance, aerial_sampler, uv, i32(high), 0.0).rgb,
        blend);
    return AerialSample(scatter, transmission);
}
fn aerial_uv(screen_position: vec4<f32>) -> vec2<f32> {
    return clamp(screen_position.xy * view_aux.viewport_inv_viewport.zw, vec2(0.0), vec2(1.0));
}
// Decode the same 1024 RGB storage gain as the Sky before applying Sun once.
fn aerial_radiance(scatter: vec3<f32>) -> vec3<f32> {
    return scatter * (aerial_frame.sun_lux.w / SKY_LUT_GAIN) * aerial_frame.colour_pre_exposure.rgb;
}
