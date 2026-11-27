    cell.uv = uv;
    cell.weights = vec4((1.0-uv.x)*(1.0-uv.y), uv.x*(1.0-uv.y), (1.0-uv.x)*uv.y, uv.x*uv.y);
    for (var k = 0u; k < 4u; k++) {
        cell.corners[k] = first + CORNERS[k];
        cell.heights[k] = textureLoad(bed_map, cell.corners[k], 0, 0).x;
    }
    cell.bed = dot(cell.heights, cell.weights);
    return cell;
}

fn map_reaches(cell: Cell, k: u32, level: f32, wet: bool) -> bool {
    return bed_reaches(cell.heights, cell.bed, cell.uv, k, level, wet);
}

fn chart_row(
    point: vec2<f32>, body: BodyRecord, own: MapRow, rows: ptr<function, array<MapRow, BODY_LIMIT>>,
) -> MapRow {
    var row = own;
    var total = 1.0;
    for (var e = 0u; e < body.layers.w; e++) {
        let segment = body.ends[e];
        let join = body.joins[e];
        let other = (*rows)[u32(join.x)];
        let delta = segment.zw - segment.xy;
        let offset = point - segment.xy;
        let distance = length(offset - delta * clamp(dot(offset, delta) / dot(delta, delta), 0.0, 1.0));
        var weight = 1.0 - smoothstep(0.0, join.y, distance);
        if join.z > 0.0 {
            let reach = 0.175 * map.origin_spacing_margin.w;
            weight = max(weight, 1.0 - smoothstep(0.0, 0.25, own.water.x - other.water.x - reach));
        }
        if weight >= 1.0 { return other; }
        let relative = weight / (1.0 - weight);
        total += relative;
        row = MapRow(mix(row.water, other.water, relative / total), mix(row.gains, other.gains, relative / total));
    }
    return row;
}

fn map_body(
    point: vec2<f32>, cell: Cell, body: BodyRecord, rows: ptr<function, array<MapRow, BODY_LIMIT>>,
) -> MapRow {
    var row = MapRow(vec3(body.level.x + body.level.y, 0.0, 0.0), body.gains);
    if body.layers.x != 2u { return row; }
    if cell.inside {
        var water = vec3(body.level.y, 0.0, 0.0);
        for (var k = 0u; k < 4u; k++) {
