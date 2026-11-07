@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var multiple_scattering: texture_2d<f32>;
@group(0) @binding(3) var scatter_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(4) var trans_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(5) var lut_sampler: sampler;

// The Sun's single-scattered part of lit_step: its multiple-scattering source is zero.
// Consumers that know terrain visibility along the ray attenuate only this part.
fn single_step(
    ray: SpectralRay, densities: vec3<f32>, phase: vec2<f32>, dt: f32,
    direct: array<vec4<f32>,2>,
) -> SpectralRay {
    let lo=scattering_step(densities,phase,dt,0u,direct[0],vec4(0.0));
    let hi=scattering_step(densities,phase,dt,1u,direct[1],vec4(0.0));
    return SpectralRay(extend_ray(ray.lo,lo),extend_ray(ray.hi,hi));
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
    let phase = scattering_phase(dot(direction, sun));
    var limit = min(32000.0, top_distance(h, direction.y));
    let ground = ground_distance(h, direction.y);
    if ground >= 0.0 { limit = min(limit, ground); }
    var ray = empty_ray();
    var single = empty_ray();
    let radiance = vec3(0.0);
    var throughput = vec3(1.0);
    // Alpha holds the cumulative luminance of the Sun's single scattering alone.
    textureStore(scatter_out, vec2<i32>(id.xy), 0, vec4(radiance * SKY_LUT_GAIN, 0.0));
    textureStore(trans_out, vec2<i32>(id.xy), 0, vec4(throughput, 1.0));
    for (var layer = 1u; layer <= 32u; layer++) {
        let start = 32000.0 * pow(f32(layer - 1u) / 32.0, 2.0);
        let end = 32000.0 * pow(f32(layer) / 32.0, 2.0);
        let segment_start = min(start, limit);
        let segment_end = min(end, limit);
        if segment_end > segment_start {
            let dt = (segment_end - segment_start) * 0.25;
            for (var step = 0u; step < 4u; step++) {
                let p = origin + direction * (segment_start + (f32(step) + 0.5) * dt);
                let ah = altitude(p);
                let densities = medium_densities(ah);
                let mu = local_sun_mu(p,sun);
                let direct = spectral_sun_pair(ah,mu,transmittance,lut_sampler);
                ray = lit_step(ray, densities, ah, mu, phase, dt, direct, multiple_scattering, lut_sampler);
                single = single_step(single, densities, phase, dt, direct);
                throughput *= exp(-surface_extinction(densities)*dt);
            }
        }
        let radiance = project_spectrum(ray.lo.radiance,ray.hi.radiance);
        let single_luminance = dot(project_spectrum(single.lo.radiance,single.hi.radiance), SPECTRUM_LUMA);
        textureStore(scatter_out, vec2<i32>(id.xy), i32(layer), vec4(radiance * SKY_LUT_GAIN, single_luminance * SKY_LUT_GAIN));
        textureStore(trans_out, vec2<i32>(id.xy), i32(layer), vec4(throughput, 1.0));
    }
}
