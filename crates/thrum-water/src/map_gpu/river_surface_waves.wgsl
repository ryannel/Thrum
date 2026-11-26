// Water still has its sampled bulk current. Near a mouth, fade the wind
// waves to the receiving sheet's phase before their geometry spans the join.
fn river_surface_motion(point: vec2<f32>, row: SurfaceRow) -> CascadeMotion {
    let time = water.carriage.w;
    let period = water.carriage.y;
    let carried = cascade_motion(point, row.current, row.track, time, period);
    let chart = map.bodies[row.body];
    if chart.joins[0].w == 0.0 { return carried; }
    for (var e = 0u; e < chart.layers.w; e++) {
        let join = chart.joins[e];
        if join.z <= 1.0 { continue; }
        let segment = chart.ends[e];
        let delta = segment.zw - segment.xy;
        let offset = point - segment.xy;
        let distance = length(offset - delta * clamp(dot(offset, delta) / dot(delta, delta), 0.0, 1.0));
        let spacing = map.origin_spacing_margin.z;
        let blend = smoothstep(2.0 * spacing, max(6.0 * spacing, join.y), distance);
        if blend >= 1.0 { continue; }
        let id = u32(join.x);
        var current = vec2(0.0);
        if map.bodies[id].layers.x == 2u {
            let cell = map_cell(point);
            var rows: array<MapRow, BODY_LIMIT>;
            for (var i = 0u; i <= id; i++) {
                rows[i] = map_body(point, cell, map.bodies[i], &rows);
            }
            current = rows[id].water.yz;
        }
        let receiving = cascade_motion(point, current, map_wave_track(point, id, current), time, period);
        return mix_cascade_motion(receiving, carried, blend);
    }
    return carried;
}
