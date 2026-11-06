struct AerialFrame {
    inverse_view_projection: mat4x4<f32>,
    camera_altitude: vec4<f32>,
    sun_lux: vec4<f32>,
    colour_pre_exposure: vec4<f32>,
    viewport: vec4<f32>,
}
@group(1) @binding(4) var aerial_scatter: texture_2d_array<f32>;
@group(1) @binding(5) var aerial_transmittance: texture_2d_array<f32>;
@group(1) @binding(6) var aerial_sampler: sampler;
@group(1) @binding(7) var<uniform> aerial_frame: AerialFrame;

fn aerial_colour(colour: vec3<f32>, world_position: vec3<f32>, screen_position: vec4<f32>) -> vec3<f32> {
    let distance = min(32000.0, length(world_position - aerial_frame.camera_altitude.xyz));
    let coordinate = sqrt(distance / 32000.0) * 32.0;
    let low = min(31u, u32(floor(coordinate)));
    let high = low + 1u;
    let first_distance = 32000.0 * pow(f32(low) / 32.0, 2.0);
    let second_distance = 32000.0 * pow(f32(high) / 32.0, 2.0);
    let blend = clamp((distance - first_distance) / max(0.001, second_distance - first_distance), 0.0, 1.0);
    let uv = clamp(screen_position.xy / aerial_frame.viewport.xy, vec2(0.0), vec2(1.0));
    let scatter = mix(
        textureSampleLevel(aerial_scatter, aerial_sampler, uv, i32(low), 0.0).rgb,
        textureSampleLevel(aerial_scatter, aerial_sampler, uv, i32(high), 0.0).rgb,
        blend);
    let transmission = mix(
        textureSampleLevel(aerial_transmittance, aerial_sampler, uv, i32(low), 0.0).rgb,
        textureSampleLevel(aerial_transmittance, aerial_sampler, uv, i32(high), 0.0).rgb,
        blend);
    return transmission * colour + scatter * aerial_frame.sun_lux.w * aerial_frame.colour_pre_exposure.rgb;
}
