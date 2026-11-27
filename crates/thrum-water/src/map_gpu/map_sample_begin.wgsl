    let cell = map_cell(point);
    var best = MapAnswer(NO_BODY, MapRow(vec3(0.0), vec4(0.0)), cell.bed, cell.inside);
    if !cell.inside {
        let sea = map.size_count_sea.w;
        if sea != NO_BODY {
            let body = map.bodies[sea];
            best.body = sea;
            best.row = MapRow(vec3(body.level.x + body.level.y, 0.0, 0.0), body.gains);
        }
        return best;
    }
    var rows: array<MapRow, BODY_LIMIT>;
    var rank = 0u;
    for (var id = 0u; id < map.size_count_sea.z; id++) {
        let body = map.bodies[id];
        let kind = body.layers.x;
        var row = map_body(point, cell, body, &rows);
        rows[id] = row; // Joins read the underlying row, before bank continuation.
