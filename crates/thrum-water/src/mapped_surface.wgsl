fn surface_body(slot: u32) -> u32 {
    return select(slot - 1u, map.size_count_sea.w, slot == 0u);
}

fn surface_row(cell: Cell, row: MapRow, body: u32, owned: bool) -> SurfaceRow {
    let depth = select(1e30, row.water.x - cell.bed, cell.inside);
    let gains = limit_gains(row.gains, depth, water.heights, map.swash.x, map.origin_spacing_margin.w);
    let record = map.bodies[body];
    let previous = row.water.x + record.level.z - (record.level.x + record.level.y);
    return SurfaceRow(row.water.x, previous, gains, select(-1e30, cell.bed, cell.inside), owned);
}
fn tail_gain(row: SurfaceRow) -> f32 { return row.gains.w; }

fn surface_opacity(row: SurfaceRow, world: vec3<f32>) -> f32 {
    // Blend the optically thin edge with the already rendered ground at the
    // same pixel. The displaced sheet still writes scene depth.
    var depth = world.y - row.bed;
    if depth > 2.0 { return 1.0; }
    // Ownership stays at rest. Thin-sheet appearance uses the bed directly
    // under the displaced point, where the opaque depth actually cuts it.
    let ground = map_cell(world.xz);
    if ground.inside { depth = world.y - ground.bed; }
    // Leave the optical edge visible at the opaque depth intersection, with
    // the bed still showing through the first part of the shallows.
    return mix(0.28, 1.0, smoothstep(0.0, 0.065, depth));
}

fn surface_vertex(point: vec2<f32>, slot: u32) -> SurfaceRow {
    let id = surface_body(slot);
    let body = map.bodies[id];
    let cell = map_cell(point);
    var row = MapRow(vec3(body.level.x + body.level.y, 0.0, 0.0), body.gains);
    if body.layers.x == 2u {
        var rows: array<MapRow, BODY_LIMIT>;
        for (var i = 0u; i <= id; i++) {
            rows[i] = map_body(point, cell, map.bodies[i], &rows);
        }
        row = rows[id];
        if cell.inside && !chart_contains(cell, body) {
            row = continued_chart(cell, body, row);
        }
    }
    return surface_row(cell, row, id, true);
}

fn surface_pixel(point: vec2<f32>, slot: u32) -> SurfaceRow {
    let id = surface_body(slot);
    let answer = map_sample(point);
    var cell: Cell;
    cell.inside = answer.has_bed;
    cell.bed = answer.bed;
    return surface_row(cell, answer.row, id, answer.body == id);
}
