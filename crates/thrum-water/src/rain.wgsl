@group(3) @binding(21) var rain_periodic: sampler;

fn rain_geometry(point: vec2<f32>, spacing: f32, row: SurfaceRow, motion: CascadeMotion) -> vec2<f32> {
    let gain = rain_gain(point, row);
    if gain <= 0.0 { return vec2(0.0); }
    var result = vec2(0.0);
    for (var band = 0u; band < 2u; band += 1u) {
        let side = rain.lengths[band];
        let resolution = side / 512.0;
        let lod = log2(max(1.0, spacing / resolution));
        let layer = i32(RAIN_FIRST_LAYER + 2u * band);
        let s = textureSampleLevel(cascades, rain_periodic, motion.departure / side, layer, lod);
        var value = vec2(s.x, s.w - dot(s.yz, motion.drift));
        if motion.blend < 1.0 {
            let other = textureSampleLevel(cascades, rain_periodic, motion.second / side, layer, lod);
            value = mix(vec2(other.x, other.w - dot(other.yz, motion.second_drift)), value, motion.blend)
                + vec2(0.0, motion.blend_rate * (s.x - other.x));
        }
        result += value;
    }
    return gain * result;
}

fn filtered_rain(uv: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>, layer: i32, footprint_scale: f32) -> FilteredLayer {
    // A grazing footprint can cover the short crests lengthwise while the
    // hardware filter runs out of anisotropy across them. Move their energy
    // continuously into the patch's coarsest moment before they can alias.
    let unresolved = smoothstep(0.018, 0.075, footprint_scale * max(length(dx), length(dy)));
    var m = FilteredLayer(vec2(0.0), vec3(0.0), vec3(0.0));
    if unresolved < 1.0 {
        m.slope = sample_scaled_field(cascades, rain_periodic, uv, layer, dx, dy, footprint_scale).yz;
        m.second = sample_scaled_field(cascades, rain_periodic, uv, layer+1, dx, dy, footprint_scale).xyz;
    }
    if unresolved > 0.0 {
        let mean_square = textureSampleLevel(cascades, rain_periodic, uv, layer+1, 9.0).xyz;
        m.slope *= 1.0 - unresolved;
        m.second = mix(m.second, mean_square, unresolved);
    }
    return m;
}

fn rain_normal(point: vec2<f32>, row: SurfaceRow, motion: CascadeMotion,
    dx: vec2<f32>, dy: vec2<f32>, second_dx: vec2<f32>, second_dy: vec2<f32>,
    slope: ptr<function, vec2<f32>>, covariance: ptr<function, vec3<f32>>) {
    let gain = rain_gain(point, row);
    if gain <= 0.0 { return; }
    let background = 0.5 * rain.lengths.z;
    *covariance += gain * gain * vec3(background, 0.0, background);
    for (var band = 0u; band < 2u; band += 1u) {
        let side = rain.lengths[band];
        let layer = i32(RAIN_FIRST_LAYER + 2u * band);
        // The resolved detail transitions at fixed physical footprints.
        // The schedule period can grow without extending costly fine sampling.
        let footprint_scale = rain.footprint_scale[band];
        var m = filtered_rain(motion.departure / side, dx / side, dy / side, layer, footprint_scale);
        if motion.blend < 1.0 {
            let other = filtered_rain(motion.second / side, second_dx / side, second_dy / side, layer, footprint_scale);
            m.slope = mix(other.slope, m.slope, motion.blend);
            m.second = mix(other.second, m.second, motion.blend);
        }
        *slope += gain * m.slope;
        *covariance += gain * gain * (m.second - vec3(m.slope.x*m.slope.x, m.slope.x*m.slope.y, m.slope.y*m.slope.y));
    }
}
