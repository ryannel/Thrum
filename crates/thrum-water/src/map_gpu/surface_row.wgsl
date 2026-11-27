fn surface_row(point: vec2<f32>, cell: Cell, row: MapRow, body: u32, owned: bool) -> SurfaceRow {
    let depth = select(1e30, row.water.x - cell.bed, cell.inside);
    let gains = limit_gains(row.gains, depth, water.heights, map.swash.x, map.origin_spacing_margin.w);
    let record = map.bodies[body];
    let previous = row.water.x + record.level.z - (record.level.x + record.level.y);
    var swash = 0.0;
    var loss = 0.0;
    var loss_without_long = 0.0;
    if FOAM_ENABLED {
        let before = row.gains * water.heights;
        swash = min(map.swash.x * length(before), map.origin_spacing_margin.w);
        loss = lost_wave_power(row.gains, gains, water.heights);
        if SURF_CACHE_ENABLED {
            loss_without_long = lost_wave_power(vec4(0.0, row.gains.yzw),
                vec4(0.0, gains.yzw), water.heights);
        }
    }
    return SurfaceRow(row.water.x, previous, gains, select(-1e30, cell.bed, cell.inside),
