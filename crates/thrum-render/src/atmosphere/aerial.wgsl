@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var multiple_scattering: texture_2d<f32>;
@group(0) @binding(3) var scatter_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(4) var trans_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(5) var lut_sampler: sampler;

// Aerial columns are interpolation support, including below the model's
// ground. The charts and the sky keep their original, clamped altitude.
fn aerial_altitude(p: vec3<f32>) -> f32 {
    let n = dot(p,p)+2.0*R*p.y;
    return n/(sqrt(max(0.0,R*R+n))+R);
}

// Continue the chart's near-surface optical-depth slope below its domain.
// Diffuse sources stay at the surface; the direct Sun alone dims with depth.
// A Sun hidden from the ground point above the sample remains hidden.
fn aerial_sun_pair(h: f32, mu: f32) -> array<vec4<f32>,2> {
    let surface = spectral_sun_pair(max(0.0,h),mu,transmittance,lut_sampler);
    if h >= 0.0 || mu < 0.0 { return surface; }
    let above = spectral_sun_pair(50.0,mu,transmittance,lut_sampler);
    let depth = vec4(-h/50.0);
    let lo = clamp(surface[0]/max(above[0],vec4(1e-20)),vec4(0.0),vec4(1.0));
    let hi = clamp(surface[1]/max(above[1],vec4(1e-20)),vec4(0.0),vec4(1.0));
    return array(surface[0]*pow(lo,depth),surface[1]*pow(hi,depth));
}

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
    let limit = min(32000.0, top_distance(h, direction.y));
    var ray = empty_ray();
    var single = empty_ray();
    let radiance = vec3(0.0);
    var throughput = vec3(1.0);
    var night_throughput = 1.0;
    var night = 0.0;
    // Alpha holds the cumulative luminance of the Sun's single scattering alone.
    textureStore(scatter_out, vec2<i32>(id.xy), 0, vec4(radiance * SKY_LUT_GAIN, 0.0));
    textureStore(trans_out, vec2<i32>(id.xy), 0, vec4(throughput, night));
    for (var layer = 1u; layer <= AERIAL_INTERVALS; layer++) {
        let start = 32000.0 * pow(f32(layer - 1u) / f32(AERIAL_INTERVALS), 2.0);
        let end = 32000.0 * pow(f32(layer) / f32(AERIAL_INTERVALS), 2.0);
        let segment_start = min(start, limit);
        let segment_end = min(end, limit);
        if segment_end > segment_start {
            if night_control.x != 0u {
                let middle = origin + direction * (0.5 * (segment_start + segment_end));
                let height = aerial_altitude(middle);
                let mu = local_sun_mu(middle, direction);
                let sigma = extinction(medium_densities(height), 1u).x;
                let source = night_source(max(0.0,height), mu, lut_sampler);
                // Use the stable midpoint attenuation form at the short first layer.
                let optical_depth = sigma * (segment_end - segment_start);
                let trans = exp(-0.5 * optical_depth);
                night += night_throughput * source * (segment_end - segment_start) * trans;
                night_throughput *= trans * trans;
            }
            let dt = (segment_end - segment_start) * 0.25;
            for (var step = 0u; step < 4u; step++) {
                let p = origin + direction * (segment_start + (f32(step) + 0.5) * dt);
                let ah = aerial_altitude(p);
                let densities = medium_densities(ah);
                let mu = local_sun_mu(p,sun);
                let direct = aerial_sun_pair(ah,mu);
                ray = lit_step(ray, densities, max(0.0,ah), mu, phase, dt, direct, multiple_scattering, lut_sampler);
                single = single_step(single, densities, phase, dt, direct);
                throughput *= exp(-surface_extinction(densities)*dt);
            }
        }
        let radiance = project_spectrum(ray.lo.radiance,ray.hi.radiance);
        let single_luminance = dot(project_spectrum(single.lo.radiance,single.hi.radiance), SPECTRUM_LUMA);
        textureStore(scatter_out, vec2<i32>(id.xy), i32(layer), vec4(radiance * SKY_LUT_GAIN, single_luminance * SKY_LUT_GAIN));
        textureStore(trans_out, vec2<i32>(id.xy), i32(layer), vec4(throughput, night));
    }
}
