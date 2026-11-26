fn reflection_bed(point: vec2<f32>) -> f32 {
    let cell = map_cell(point);
    return select(-1e30, cell.bed, cell.inside);
}

fn surface_body(slot: u32) -> u32 {
    return select(slot - 1u, map.size_count_sea.w, slot == 0u);
}

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
        owned, swash, loss, loss_without_long, row.water.yz, map_wave_track(point, body, row.water.yz));
}
fn tail_gain(row: SurfaceRow) -> f32 { return row.gains.w; }

fn optical_weights(t: f32) -> vec4<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    return vec4(-t + 2.0*t2 - t3, 2.0 - 5.0*t2 + 3.0*t3,
                t + 4.0*t2 - 3.0*t3, -t2 + t3) * 0.5;
}

fn optical_bed(cell: Cell) -> f32 {
    // Interpolate the bed across cell edges for appearance. The map's
    // bilinear bed still owns wave support and opaque depth cuts the line.
    let last = vec2<i32>(map.size_count_sea.xy) - vec2(1);
    let wx = optical_weights(cell.uv.x);
    let wz = optical_weights(cell.uv.y);
    var rows: vec4<f32>;
    for (var z = 0; z < 4; z++) {
        var heights: vec4<f32>;
        for (var x = 0; x < 4; x++) {
            let at = clamp(cell.corners[0] + vec2(x - 1, z - 1), vec2(0), last);
            heights[x] = textureLoad(bed_map, at, 0, 0).x;
        }
        rows[z] = dot(heights, wx);
    }
    return dot(rows, wz);
}

fn surface_opacity(row: SurfaceRow, world: vec3<f32>, view_y: f32) -> f32 {
    // Blend the optically thin edge with the already rendered ground at the
    // same pixel. The displaced sheet still writes scene depth.
    var depth = world.y - row.bed;
    if depth > 2.0 { return 1.0; }
    // Ownership stays at rest. Thin-sheet appearance uses the bed directly
    // under the displaced point, where the opaque depth actually cuts it.
    let ground = map_cell(world.xz);
    var swash = 0.0;
    if ground.inside {
        let bed = optical_bed(ground);
        depth = world.y - bed;
        if SURF_CACHE_ENABLED && row.swash > 0.0 {
            swash = 1.0 - smoothstep(0.25 * row.swash, 0.65 * row.swash, row.level - bed);
        }
    }
    // A glancing ray travels farther through the thin sheet. Let the ground
    // contribution decay continuously with that path to the bed.
    let path = max(0.0, depth) / max(0.15, abs(view_y));
    let opacity = 1.0 - 0.72 * exp2(-mix(27.0, 13.0, swash) * path);
    // A still interface has a readable reflection at contact. Moving swash
    // loses visible coverage as the advancing sheet runs out of thickness.
    let agitation = smoothstep(0.0, 0.4, length(water.heights));
    // Keep the breaker trough reflective. The bounded swash shows the sand
    // through the thinning sheet as it approaches the wet edge.
    let edge_depth = select(0.12, mix(0.055, 0.12, swash), SURF_CACHE_ENABLED);
    return opacity * mix(1.0, smoothstep(0.0, edge_depth, depth), agitation);
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
    return surface_row(point, cell, row, id, true);
}

fn surface_pixel(point: vec2<f32>, slot: u32) -> SurfaceRow {
    let id = surface_body(slot);
    let answer = map_sample(point);
    var cell: Cell;
    cell.inside = answer.has_bed;
    cell.bed = answer.bed;
    return surface_row(point, cell, answer.row, id, answer.body == id);
}
