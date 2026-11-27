// Flow gains and power follow the same sample brackets, wetting thresholds
// and join weights as the immutable level and current fields.
fn chart_flow(cell: Cell, body: BodyRecord, first: u32) -> vec3<f32> {
    if first == 0u || !cell.inside { return vec3(0.0); }
    let upper = i32(body.joins[0].w);
    let interval = max(0, upper - i32(body.level.w));
    var result = vec3(0.0);
    for (var k = 0u; k < 4u; k++) {
        let corner = cell.corners[k];
        let lo = textureLoad(bed_map, corner, i32(first) + interval, 0).xyz;
        var value = lo;
        if upper != 0 {
            let hi = textureLoad(bed_map, corner, i32(first) + interval + 1, 0).xyz;
            let wetting = textureLoad(bed_map, corner, upper, 0).w;
            let weight = body.joins[1].w;
            if wetting < 0.0 {
                value = mix(lo, hi, weight);
            } else if weight <= wetting && weight < 1.0 {
                value = mix(lo, vec3(0.0), select(1.0, weight / wetting, wetting > 0.0));
            } else {
                value = hi * select(1.0, (weight - wetting) / (1.0 - wetting), wetting < 1.0);
            }
        }
        result += value * cell.weights[k];
    }
    return result;
}

fn flow_join_weight(point: vec2<f32>, body: BodyRecord, e: u32, own_level: f32, other_level: f32) -> f32 {
    let join = body.joins[e];
    let segment = body.ends[e];
    let delta = segment.zw - segment.xy;
    let offset = point - segment.xy;
    let distance = length(offset - delta * clamp(dot(offset, delta) / dot(delta, delta), 0.0, 1.0));
    var weight = 1.0 - smoothstep(0.0, join.y, distance);
    if join.z > 0.0 {
        let reach = 0.175 * map.origin_spacing_margin.w;
        weight = max(weight, 1.0 - smoothstep(0.0, 0.25, own_level - other_level - reach));
    }
    return weight;
}

fn chart_flow_level(cell: Cell, body: BodyRecord) -> f32 {
    var level = body.level.y;
    for (var k = 0u; k < 4u; k++) {
        level += river_chart_cell(cell, k, body).x * cell.weights[k];
    }
    return level;
}

fn map_flow(point: vec2<f32>, id: u32) -> vec3<f32> {
    if id == NO_BODY || map.bodies[id].layers.x != 2u { return vec3(0.0); }
    let cell = map_cell(point);
    if !cell.inside { return vec3(0.0); }
    let body = map.bodies[id];
    var joined_chart = false;
    for (var e = 0u; e < body.layers.w; e++) {
        joined_chart |= map.bodies[u32(body.joins[e].x)].layers.x == 2u;
    }
    // A cut into a lake or sea has no incoming flow layers. It needs only
    // its own chart samples and the receiving level for a stage-controlled mouth.
    if !joined_chart {
        let local = chart_flow(cell, body, map.flow_layers[id].x);
        if all(local == vec3(0.0)) { return local; }
        var total = 1.0;
        for (var e = 0u; e < body.layers.w; e++) {
            let receiver = map.bodies[u32(body.joins[e].x)];
            var own = 0.0;
            if body.joins[e].z > 0.0 { own = chart_flow_level(cell, body); }
            let weight = flow_join_weight(point, body, e, own, receiver.level.x + receiver.level.y);
            if weight >= 1.0 { return vec3(0.0); }
            total += weight / (1.0 - weight);
        }
        return local / total;
    }
    var flows: array<vec3<f32>, BODY_LIMIT>;
    var rows: array<MapRow, BODY_LIMIT>;
    for (var i = 0u; i <= id; i++) {
        let reach = map.bodies[i];
        if reach.layers.x != 2u {
            rows[i] = MapRow(vec3(reach.level.x + reach.level.y, 0.0, 0.0), reach.gains);
            continue;
        }
        rows[i] = map_body(point, cell, reach, &rows);
        var value = chart_flow(cell, reach, map.flow_layers[i].x);
        var total = 1.0;
        for (var e = 0u; e < reach.layers.w; e++) {
            let other = u32(reach.joins[e].x);
            var own = 0.0;
            if reach.joins[e].z > 0.0 { own = chart_flow_level(cell, reach); }
            let weight = flow_join_weight(point, reach, e, own, rows[other].water.x);
            if weight >= 1.0 {
                value = flows[other];
                break;
            }
            let relative = weight / (1.0 - weight);
            total += relative;
            value = mix(value, flows[other], relative / total);
        }
        flows[i] = value;
    }
    return flows[id];
}
