        let level = row.water.x;
        if cell.bed > level + map.origin_spacing_margin.w { continue; }
        var supported = chart_water;
        for (var k = 0u; k < 4u && !supported; k++) {
            if cell.weights[k] <= 0.0 { continue; }
            let source = textureLoad(bed_map, cell.corners[k], i32(body.layers.y), 0);
            if source.w == 0.0 { continue; }
            let source_level = select(level, source.x, kind == 2u);
            supported = map_reaches(cell, k, source_level, kind == 1u && cell.bed < level);
        }
        if !supported { continue; }
        let old = best.row.water.x;
        let both_margin = cell.bed >= max(level, old);
        if best.body == NO_BODY || select(kind > rank || (kind == rank && level > old), level > old, both_margin) {
            best.body = id;
            best.row = row;
            rank = kind;
        }
    }
    return best;
}

// The extra chart layer holds either the bounded reference and authored
// velocities or the parallel track's section coordinate and unjoined speed.
fn map_wave_track(point: vec2<f32>, body: u32, current: vec2<f32>) -> vec4<f32> {
    if body == NO_BODY { return vec4(0.0); }
    let record = map.bodies[body];
    if record.track.w < 0.0 {
        if all(current == vec2(0.0)) { return vec4(0.0); }
        let last = vec2<f32>(map.size_count_sea.xy - vec2(1u));
        let grid = clamp((point - map.origin_spacing_margin.xy) / map.origin_spacing_margin.z,
            vec2(0.0), last);
        let first = vec2<i32>(min(floor(grid), last - vec2(1.0)));
        let uv = grid - vec2<f32>(first);
        let layer = i32(record.track.x);
        // Bounded wave reference.
    }
    let axis_code = record.track.y;
    if axis_code == 0.0 || all(current == vec2(0.0)) { return vec4(0.0); }
    let axis = u32(axis_code) - 1u;
    let last = map.size_count_sea[axis] - 1u;
    let position = clamp((point[axis] - map.origin_spacing_margin[axis])
        / map.origin_spacing_margin.z, 0.0, f32(last));
    let cell = min(u32(position), last - 1u);
    var lower = vec2<i32>(0);
    lower[axis] = i32(cell);
    var upper = lower;
    upper[axis] += 1;
    let layer = i32(record.layers.z) + 2;
    let a = textureLoad(bed_map, lower, layer, 0).xy;
    let b = textureLoad(bed_map, upper, layer, 0).xy;
    let route = mix(a, b, position - f32(cell));
    let blend = clamp(abs(current[axis]) / route.y, 0.0, 1.0);
    return vec4(route.x, record.track.x, blend, axis_code);
}
