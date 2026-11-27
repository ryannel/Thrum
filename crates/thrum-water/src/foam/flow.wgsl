fn foam_flow_height(point: vec2<f32>, current: vec2<f32>, track: vec4<f32>, gains: vec2<f32>, spacing: f32) -> f32 {
    if all(gains <= vec2(0.0)) { return 0.0; }
    let motion = cascade_motion(point, current, track, water.carriage.w, water.carriage.y);
    var height = 0.0;
    for (var band = 0; band < 2; band++) {
        if gains[band] <= 0.0 { continue; }
        let length_m = FLOW_PATCHES[band];
        let lod = clamp(log2(max(1.0, spacing * 256.0 / length_m)), 0.0, 8.0);
        let layer = FLOW_FIRST + 5 * band;
        let a = foam_flow_sample(motion.departure / length_m, layer, lod);
        var value = a;
        if motion.blend < 1.0 {
            value = mix(foam_flow_sample(motion.second / length_m, layer, lod), a, motion.blend);
        }
        height += gains[band] * value;
    }
    return height;
}

fn foam_flow_sample(uv: vec2<f32>, layer: i32, lod: f32) -> f32 {
    let velocity = textureSampleLevel(cascades, periodic, uv, layer + 2, lod);
    return textureSampleLevel(cascades, periodic, uv, layer, lod).y
        + textureSampleLevel(cascades, periodic, uv, layer + 4, lod).z
        + water.cache[0].w * (velocity.y + velocity.w);
}
