
@compute @workgroup_size(AERIAL_WORKGROUP_EDGE, AERIAL_WORKGROUP_EDGE)
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
        let single_luminance = dot(project_spectrum(single.lo.radiance,single.hi.radiance), REC709_LUMA);
        textureStore(scatter_out, vec2<i32>(id.xy), i32(layer), vec4(radiance * SKY_LUT_GAIN, single_luminance * SKY_LUT_GAIN));
        textureStore(trans_out, vec2<i32>(id.xy), i32(layer), vec4(throughput, night));
    }
}
