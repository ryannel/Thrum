fn flow_height_rate(place: vec2<f32>, drift: vec2<f32>, scale: f32, layer: i32, lod: f32) -> vec2<f32> {
    let uv = place / scale;
    let height = textureSampleLevel(cascades, periodic, uv, layer, lod).y
        + textureSampleLevel(cascades, periodic, uv, layer + 4, lod).z;
    let slope = textureSampleLevel(cascades, periodic, uv, layer + 1, lod).xy;
    let velocity = textureSampleLevel(cascades, periodic, uv, layer + 2, lod);
    let rate = velocity.y + velocity.w;
    // The field ages at a fixed rest coordinate. Carry its samples to their
    // current departure points before applying the Eulerian derivative.
    return vec2(height + water.cache[0].w * rate, rate - dot(slope, drift));
}

fn surface_flow(spacing: f32, row: SurfaceRow, motion: CascadeMotion) -> vec2<f32> {
    let gains = row.flow_gains;
    var result = vec2(0.0);
    for (var band = 0; band < 2; band++) {
        if gains[band] <= 0.0 { continue; }
        let scale = FLOW_PATCHES[band];
        let lod = clamp(log2(max(1.0, spacing * 256.0 / scale)), 0.0, 8.0);
        let layer = FLOW_FIRST + band * 5;
        let a = flow_height_rate(motion.departure, motion.drift, scale, layer, lod);
        var value = a;
        if motion.blend < 1.0 {
            let b = flow_height_rate(motion.second, motion.second_drift, scale, layer, lod);
            value = mix(b, a, motion.blend);
            value.y += motion.blend_rate * (a.x - b.x);
        }
        result += gains[band] * value;
    }
    return result;
}
