
fn river_column_level(point: vec2<f32>, spacing: f32, row: SurfaceRow) -> f32 {
    let motion = river_surface_motion(point, row);
    var height = river_swell(point, spacing, row) + wake_at(point, 0.0).x;
    // The short-wave cascades are sampled at the fixed column, as in the
    // body query. The mouth still shares the receiver's wave phase.
    for (var band = 0u; band < 3u; band++) {
        let gain = row.gains[band + 1u];
        if gain * water.heights[band + 1u] > 0.0 {
            height += gain * transported_field(motion, band).displacement.y;
        }
    }
    height += surface_flow(0.0, row, motion).x;
    return row.level + height + river_rain_height(point, spacing, row, motion);
}

fn river_refinement(point: vec2<f32>, id: u32, spacing: f32) -> f32 {
    if map.flow_layers[id].x != 0u || map.bodies[id].joins[0].w > 0.0 { return 1.0; }
    if spacing > 2.0 { return 0.0; }
    var weight = 0.0;
    // A receiving sheet shares the correction at the exact ownership edge.
    // Fade to its ordinary mesh beyond the strip that spans that edge.
    for (var i = id + 1u; i < map.size_count_sea.z; i++) {
        let chart = map.bodies[i];
        if chart.joins[0].w == 0.0 { continue; }
        for (var e = 0u; e < chart.layers.w; e++) {
            if u32(chart.joins[e].x) != id { continue; }
            let segment = chart.ends[e];
            let delta = segment.zw - segment.xy;
            let offset = point - segment.xy;
            let distance = length(offset - delta * clamp(dot(offset, delta) / dot(delta, delta), 0.0, 1.0));
            weight = max(weight, 1.0 - smoothstep(spacing, 3.0 * spacing, distance));
        }
    }
    return weight;
}
