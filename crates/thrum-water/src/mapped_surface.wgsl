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

fn shallow_colour(row: SurfaceRow, world: vec3<f32>, facing: f32, incident: vec3<f32>, fresnel: f32, light: vec3<f32>) -> vec3<f32> {
    // A diffuse, authored bed proxy fades through the thin sheet; actual
    // transmitted scene colour and refracted depth belong to the gather.
    var depth = world.y - row.bed;
    if all(water.shoal_bed.rgb == vec3(0.0)) || depth > 3.0 { return light; }
    // Ownership stays at rest. Thin-sheet appearance uses the bed directly
    // under the displaced point, where the opaque depth actually cuts it.
    let ground = map_cell(world.xz);
    if ground.inside { depth = world.y - ground.bed; }
    let path = max(0.0, depth) / max(0.36, facing);
    let transmission = exp(-path * vec3(4.0, 2.8, 2.2));
    let reflected_bed = incident * water.shoal_bed.rgb / 3.14159265;
    let deep_scatter = incident * water.scatter.rgb / 3.14159265;
    let water_colour = light + (1.0-fresnel) * (reflected_bed - deep_scatter) * transmission;
    return mix(reflected_bed, water_colour, smoothstep(0.0, 0.12, depth));
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
