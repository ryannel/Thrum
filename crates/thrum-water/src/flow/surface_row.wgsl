fn surface_row(point: vec2<f32>, cell: Cell, row: MapRow, body: u32, owned: bool) -> SurfaceRow {
    let depth = select(1e30, row.water.x - cell.bed, cell.inside);
    let raw_flow = map_flow(point, body).xy;
    let swash = flow_swatch(row.gains * water.heights, raw_flow, map.swash.x, map.origin_spacing_margin.w);
    let gains = limit_wind_with_flow(row.gains, water.heights, depth, swash);
    let record = map.bodies[body];
    let previous = row.water.x + record.level.z - (record.level.x + record.level.y);
    var loss = 0.0;
    var loss_without_long = 0.0;
    if FOAM_ENABLED {
        loss = lost_wave_power(row.gains, gains, water.heights);
        if SURF_CACHE_ENABLED {
            loss_without_long = lost_wave_power(vec4(0.0, row.gains.yzw),
                vec4(0.0, gains.yzw), water.heights);
        }
    }
    return SurfaceRow(row.water.x, previous, gains, select(-1e30, cell.bed, cell.inside),
