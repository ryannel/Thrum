@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var multiple_scattering: texture_2d<f32>;
@group(0) @binding(3) var scatter_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(4) var trans_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(5) var lut_sampler: sampler;

fn solar_transmittance(p: vec3<f32>, sun: vec3<f32>) -> vec3<f32> {
    let h = altitude(p);
    let up = normalize(p + vec3(0.0, R, 0.0));
    let mu = dot(up, sun);
    if blocked(h, mu) { return vec3(0.0); }
    return textureSampleLevel(transmittance, lut_sampler, trans_uv(h, mu), 0.0).rgb;
}

@compute @workgroup_size(8, 8)
fn aerial_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(scatter_out);
    if any(id.xy >= size) { return; }
    let uv = (vec2<f32>(id.xy) + 0.5) / vec2<f32>(size);
    let ndc = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let far = atmosphere.inverse_view_projection * vec4(ndc, 0.0, 1.0);
    let direction = normalize(far.xyz / far.w - atmosphere.camera_altitude.xyz);
    let h = atmosphere.camera_altitude.w;
    let origin = vec3(0.0, h, 0.0);
    let sun = atmosphere.sun_lux.xyz;
    let cosine = dot(direction, sun);
    let rayleigh_phase = 3.0 * (1.0 + cosine * cosine) / (16.0 * PI);
    let g = 0.8;
    let mie_phase = (1.0 - g * g) / (4.0 * PI * pow(max(0.001, 1.0 + g * g - 2.0 * g * cosine), 1.5));
    var limit = min(32000.0, top_distance(h, direction.y));
    let ground = ground_distance(h, direction.y);
    if ground >= 0.0 { limit = min(limit, ground); }
    var radiance = vec3(0.0);
    var throughput = vec3(1.0);
    textureStore(scatter_out, vec2<i32>(id.xy), 0, vec4(radiance, 1.0));
    textureStore(trans_out, vec2<i32>(id.xy), 0, vec4(throughput, 1.0));
    for (var layer = 1u; layer <= 32u; layer++) {
        let start = 32000.0 * pow(f32(layer - 1u) / 32.0, 2.0);
        let end = 32000.0 * pow(f32(layer) / 32.0, 2.0);
        let segment_start = min(start, limit);
        let segment_end = min(end, limit);
        let dt = (segment_end - segment_start) * 0.25;
        for (var step = 0u; step < 4u; step++) {
            let p = origin + direction * (segment_start + (f32(step) + 0.5) * dt);
            let ah = altitude(p);
            let ext = extinction(ah);
            let single = (vec3(5.802e-6, 13.558e-6, 33.100e-6) * exp(-ah / 8000.0) * rayleigh_phase
                + vec3(3.996e-6) * exp(-ah / 1200.0) * mie_phase) * solar_transmittance(p, sun);
            let sun_mu = dot(normalize(p + vec3(0.0, R, 0.0)), sun);
            let multiple = textureSampleLevel(multiple_scattering, lut_sampler, vec2(sun_mu * 0.5 + 0.5, ah / H), 0.0).rgb;
            radiance += throughput * safe_segment(single + scattering(ah) * multiple, ext, dt);
            throughput *= exp(-ext * dt);
        }
        textureStore(scatter_out, vec2<i32>(id.xy), i32(layer), vec4(radiance, 1.0));
        textureStore(trans_out, vec2<i32>(id.xy), i32(layer), vec4(throughput, 1.0));
    }
}
