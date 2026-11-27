@group(0) @binding(9) var history_lo_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(10) var history_hi_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(11) var history_lo: texture_2d_array<f32>;
@group(0) @binding(12) var history_hi: texture_2d_array<f32>;
@group(0) @binding(15) var<uniform> volume_history: VolumeHistory;

// Only the shadowed solar source needs temporal filtering in air alone.
// Extinction and the sky source are deterministic at the cell centre. Keep
// a source density, rather than accumulated in-scatter, in the history.
fn volume_source_history(point: vec3<f32>, lo: vec4<f32>, hi: vec4<f32>) -> array<vec4<f32>, 2> {
    let fresh = array(lo, hi);
    if volume_history.previous_eye_valid.w == 0.0 { return fresh; }
    let clip = volume_history.previous_view_projection * vec4(point, 1.0);
    if clip.w <= 0.0 || any(abs(clip.xy) >= vec2(clip.w)) { return fresh; }
    let distance = length(point - volume_history.previous_eye_valid.xyz);
    if distance >= 32000.0 { return fresh; }
    let uv = clip.xy / clip.w * vec2(0.5, -0.5) + vec2(0.5);
    let coordinate = max(0.0, volume_coordinate(distance) - 0.5);
    let lower = i32(floor(coordinate));
    let upper = min(lower + 1, i32(VOLUME_INTERVALS - 1u));
    let t = fract(coordinate);
    let old_lo = mix(textureSampleLevel(history_lo, lut_sampler, uv, lower, 0.0),
        textureSampleLevel(history_lo, lut_sampler, uv, upper, 0.0), t);
    let old_hi = mix(textureSampleLevel(history_hi, lut_sampler, uv, lower, 0.0),
        textureSampleLevel(history_hi, lut_sampler, uv, upper, 0.0), t);
    // The signed source uses the same gain on both sides before fp16 storage.
    // A majority of the current measurement keeps history responsive at cuts
    // and disocclusions, while filtering the per-frame shadow sample.
    return array(mix(lo, old_lo, 0.40), mix(hi, old_hi, 0.40));
}

@compute @workgroup_size(AERIAL_WORKGROUP_EDGE, AERIAL_WORKGROUP_EDGE)
fn view_volume_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(scatter_out);
    if any(id.xy >= size) { return; }
    let xy = vec2<i32>(id.xy);
    let uv = (vec2<f32>(id.xy) + 0.5) / vec2<f32>(size);
    let ndc = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let far = atmosphere.inverse_view_projection * vec4(ndc, 0.0, 1.0);
    let eye = atmosphere.camera_altitude.xyz;
    let direction = normalize(far.xyz / far.w - eye);
    let h = atmosphere.camera_altitude.w;
    let origin = vec3(0.0, h, 0.0);
    let sun = atmosphere.sun_lux.xyz;
    let phase = scattering_phase(dot(direction, sun));
    let limit = min(32000.0, top_distance(h, direction.y));
    let cloud_projection = volume_cloud_projection(direction);
    var baseline = empty_ray();
    var joint_medium = empty_ray();
    var relative = 1.0;
    var has_medium = false;
    var correction = empty_ray();
    var throughput = vec3(1.0);
    var night_throughput = 1.0;
    var baseline_night_throughput = 1.0;
    var baseline_night = 0.0;
    var ambient = 0.0;
    var night = 0.0;
    textureStore(scatter_out, xy, 0, vec4(0.0));
    textureStore(trans_out, xy, 0, vec4(throughput, night));

    for (var layer = 0u; layer < VOLUME_INTERVALS; layer++) {
        let start = min(volume_distance(layer), limit);
        let end = min(volume_distance(layer + 1u), limit);
        let delta = end - start;
        let middle = 0.5 * (start + end);
        var source = array(vec4(0.0), vec4(0.0));
        if delta > 0.0 {
            let point = origin + direction * middle;
            let altitude = aerial_altitude(point);
            let densities = medium_densities(altitude);
            let mu = local_sun_mu(point, sun);
            let direct = aerial_sun_pair(altitude, mu);
            let uv = aerosol_multiple_uv(altitude, mu);
            let jitter = volume_shadow_jitter(id.xy, layer, volume_history.current_view_forward_phase.w);
            let sample_distance = mix(start, end, jitter);
            let world = eye + direction * sample_distance;
            let terrain = volume_terrain_visibility(world, sample_distance)
                * volume_ground_visibility(world, sample_distance);
            let visibility = terrain
                * volume_cloud_visibility(eye, direction, cloud_projection, start, end);
            let sigma_lo = extinction(densities, 0u);
            let sigma_hi = extinction(densities, 1u);
            let rayleigh_lo = RAYLEIGH_BETA[0] * densities.x;
            let rayleigh_hi = RAYLEIGH_BETA[1] * densities.x;
            let mie_lo = aerosol.scattering[0] * densities.y;
            let mie_hi = aerosol.scattering[1] * densities.y;
            let single_lo = (rayleigh_lo * phase.x + mie_lo * phase.y) * direct[0];
            let single_hi = (rayleigh_hi * phase.x + mie_hi * phase.y) * direct[1];
            let multi_lo = (rayleigh_lo + mie_lo) * spectral_lut(uv, 0u, multiple_scattering, lut_sampler) / MULTIPLE_LUT_GAIN;
            let multi_hi = (rayleigh_hi + mie_hi) * spectral_lut(uv, 1u, multiple_scattering, lut_sampler) / MULTIPLE_LUT_GAIN;
            let change = (visibility - 1.0) * VOLUME_HISTORY_GAIN;
            source = volume_source_history(eye + direction * middle, single_lo * change, single_hi * change);
            // Clamp stale source density to the amount of sunlight available in
            // the new cell. Air above a shadow can never gain negative sunlight.
            source[0] = clamp(source[0], -single_lo * VOLUME_HISTORY_GAIN, vec4(0.0));
            source[1] = clamp(source[1], -single_hi * VOLUME_HISTORY_GAIN, vec4(0.0));
            var medium = vec3(0.0);
            if VOLUME_HAS_MEDIUM {
                medium = medium_cell(xy, layer, eye + direction * middle);
                if !has_medium && any(medium != vec3(0.0)) {
                    has_medium = true;
                    joint_medium.lo = Segment(baseline.lo.radiance + correction.lo.radiance, baseline.lo.transmission);
                    joint_medium.hi = Segment(baseline.hi.radiance + correction.hi.radiance, baseline.hi.transmission);
                }
                if has_medium {
                    joint_medium.lo = extend_ray(joint_medium.lo, joint_step(single_lo + multi_lo + source[0] / VOLUME_HISTORY_GAIN, sigma_lo, direct[0], medium, terrain, delta));
                    joint_medium.hi = extend_ray(joint_medium.hi, joint_step(single_hi + multi_hi + source[1] / VOLUME_HISTORY_GAIN, sigma_hi, direct[1], medium, terrain, delta));
                }
                relative *= exp(-medium.x * delta);
            }
            baseline.lo = extend_ray(baseline.lo, integrate_segment(single_lo + multi_lo, sigma_lo, delta));
            baseline.hi = extend_ray(baseline.hi, integrate_segment(single_hi + multi_hi, sigma_hi, delta));
            correction.lo = extend_ray(correction.lo, integrate_segment(source[0] / VOLUME_HISTORY_GAIN, sigma_lo, delta));
            correction.hi = extend_ray(correction.hi, integrate_segment(source[1] / VOLUME_HISTORY_GAIN, sigma_hi, delta));
            throughput *= exp(-surface_extinction(densities) * delta);
            if night_control.x != 0u || VOLUME_HAS_MEDIUM {
                let sigma = sigma_hi.x + medium.x;
                let trans = exp(-0.5 * sigma * delta);
                if night_control.x != 0u {
                    let source = aerosol_night_source(max(0.0, altitude),
                        local_sun_mu(point, direction), lut_sampler) * delta;
                    night += night_throughput * source * trans;
                    if VOLUME_HAS_MEDIUM {
                        let clear_trans = exp(-0.5 * sigma_hi.x * delta);
                        baseline_night += baseline_night_throughput * source * clear_trans;
                        baseline_night_throughput *= clear_trans * clear_trans;
                    }
                }
                if VOLUME_HAS_MEDIUM && medium.z != 0.0 {
                    let integral = (1.0 - trans * trans) / max(sigma, 1e-12);
                    ambient += night_throughput * medium.z * integral;
                }
                night_throughput *= trans * trans;
            }
        }
        textureStore(history_lo_out, xy, i32(layer), source[0]);
        textureStore(history_hi_out, xy, i32(layer), source[1]);
        var joint = project_spectrum(baseline.lo.radiance + correction.lo.radiance,
            baseline.hi.radiance + correction.hi.radiance);
        if has_medium { joint = project_spectrum(joint_medium.lo.radiance, joint_medium.hi.radiance); }
        textureStore(scatter_out, xy, i32(layer + 1u), vec4(joint * SKY_LUT_GAIN, ambient));
        textureStore(trans_out, xy, i32(layer + 1u), vec4(throughput * relative, night));
    }
    let baseline_rgb = project_spectrum(baseline.lo.radiance, baseline.hi.radiance);
    var joint_rgb = project_spectrum(baseline.lo.radiance + correction.lo.radiance,
        baseline.hi.radiance + correction.hi.radiance);
    if has_medium { joint_rgb = project_spectrum(joint_medium.lo.radiance, joint_medium.hi.radiance); }
    let terminal = vec4((joint_rgb - relative * baseline_rgb) * SKY_LUT_GAIN, relative);
    let terminal_layer = i32(VOLUME_INTERVALS + 1u);
    textureStore(scatter_out, xy, terminal_layer, terminal);
    var diffuse = 0.0;
    if VOLUME_HAS_MEDIUM { diffuse = ambient + (night - relative * baseline_night) / NIGHT_LUT_GAIN; }
    textureStore(trans_out, xy, terminal_layer, vec4(0.0, 0.0, 0.0, diffuse));
}

fn joint_step(source: vec4<f32>, sigma: vec4<f32>, direct: vec4<f32>,
              medium: vec3<f32>, sun_visibility: f32, delta: f32) -> Segment {
    return integrate_segment(source + medium.y * direct * sun_visibility, sigma + vec4(medium.x), delta);
}
