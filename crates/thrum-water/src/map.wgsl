struct BodyRecord {
    level: vec4<f32>,
    gains: vec4<f32>,
    layers: vec4<u32>,
    ends: array<vec4<f32>, 2>,
    joins: array<vec4<f32>, 2>,
    track: vec4<f32>, // parallel reference and axis, or bounded layer, period limit and mode
}
struct MapRecord {
    origin_spacing_margin: vec4<f32>,
    size_count_sea: vec4<u32>,
    swash: vec4<f32>,
    bodies: array<BodyRecord, BODY_LIMIT>,
}
@group(WATER_MAP_GROUP) @binding(5) var bed_map: texture_2d_array<f32>;
@group(WATER_MAP_GROUP) @binding(6) var<uniform> map: MapRecord;

struct Cell {
    corners: array<vec2<i32>, 4>,
    weights: vec4<f32>,
    heights: vec4<f32>,
    uv: vec2<f32>,
    bed: f32,
    inside: bool,
}
struct MapRow {
    water: vec3<f32>, // level, current in x/z
    gains: vec4<f32>,
}
struct MapAnswer {
    body: u32,
    row: MapRow,
    bed: f32,
    has_bed: bool,
}
const NO_BODY = 0xffffffffu;
const CORNERS = array(vec2(0, 0), vec2(1, 0), vec2(0, 1), vec2(1, 1));
fn map_cell(point: vec2<f32>) -> Cell {
    let last = vec2<f32>(map.size_count_sea.xy - vec2(1u));
    let grid = (point - map.origin_spacing_margin.xy) / map.origin_spacing_margin.z;
    let inside = all(grid >= vec2(0.0)) && all(grid <= last);
    var cell: Cell;
    cell.inside = inside;
    if !inside { return cell; }
    let first = vec2<i32>(min(floor(grid), last - vec2(1.0)));
    let uv = grid - vec2<f32>(first);
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
    let h = cell.heights;
    let corner = vec2<f32>(CORNERS[k]);
    let uv = cell.uv - corner;
    let cross = h.w - h.z - h.y + h.x;
    let a = h[k];
    let b = uv.x * (h.y - h.x + cross * corner.y);
    let c = uv.y * (h.z - h.x + cross * corner.x);
    let d = cross * uv.x * uv.y;
    let descending = b + d < 0.0 && c + d < 0.0;
    let saddle = descending && b > 0.0 && c > 0.0;
    if wet {
        return a < level && cell.bed < level && !(saddle && a - b*c/d >= level);
    }
    let end = a + b + c + d;
    if max(end, level) < max(a, level) || (a > level && b < 0.0 && c < 0.0) {
        return false;
    }
    return !(descending && end > level) && !(saddle && a - b*c/d > level);
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
            let raw = textureLoad(bed_map, cell.corners[k], i32(body.layers.z), 0);
            water += raw.xyz * cell.weights[k];
        }
        row.water = water;
    }
    return chart_row(point, body, row, rows);
}

fn chart_contains(cell: Cell, body: BodyRecord) -> bool {
    for (var k = 0u; k < 4u; k++) {
        if cell.weights[k] > 1e-12 && textureLoad(bed_map, cell.corners[k], i32(body.layers.z), 0).w == 0.0 {
            return false;
        }
    }
    return true;
}

fn continued_chart(cell: Cell, body: BodyRecord, own: MapRow) -> MapRow {
    var row = own;
    var total = 0.0;
    for (var k = 0u; k < 4u; k++) {
        let water = textureLoad(bed_map, cell.corners[k], i32(body.layers.y), 0);
        let weight = cell.weights[k];
        if weight > 0.0 && water.w > 0.0 {
            total += weight;
            let gains = textureLoad(bed_map, cell.corners[k], i32(body.layers.z + 1u), 0);
            row = MapRow(mix(row.water, water.xyz, weight / total), mix(row.gains, gains, weight / total));
        }
    }
    return row;
}

fn map_sample(point: vec2<f32>) -> MapAnswer {
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
        var chart_water = false;
        if kind == 2u {
            if chart_contains(cell, body) {
                chart_water = cell.bed < row.water.x;
            } else {
                row = continued_chart(cell, body, row);
                if cell.bed < row.water.x { continue; }
            }
        }
        let level = row.water.x;
        if cell.bed > level + map.origin_spacing_margin.w { continue; }
        var supported = chart_water;
        for (var k = 0u; k < 4u && !supported; k++) {
            if cell.weights[k] <= 0.0 { continue; }
            let source = textureLoad(bed_map, cell.corners[k], i32(body.layers.y), 0);
            if source.w == 0.0 { continue; }
            let source_level = select(level, source.x, kind == 2u);
            supported = map_reaches(cell, k, source_level, kind == 1u && cell.bed < level);
        }
        if !supported { continue; }
        let old = best.row.water.x;
        let both_margin = cell.bed >= max(level, old);
        if best.body == NO_BODY || select(kind > rank || (kind == rank && level > old), level > old, both_margin) {
            best.body = id;
            best.row = row;
            rank = kind;
        }
    }
    return best;
}

// The extra chart layer holds either the bounded reference and authored
// velocities or the parallel track's section coordinate and unjoined speed.
fn map_wave_track(point: vec2<f32>, body: u32, current: vec2<f32>) -> vec4<f32> {
    if body == NO_BODY { return vec4(0.0); }
    let record = map.bodies[body];
    if record.track.w < 0.0 {
        if all(current == vec2(0.0)) { return vec4(0.0); }
        let last = vec2<f32>(map.size_count_sea.xy - vec2(1u));
        let grid = clamp((point - map.origin_spacing_margin.xy) / map.origin_spacing_margin.z,
            vec2(0.0), last);
        let first = vec2<i32>(min(floor(grid), last - vec2(1.0)));
        let uv = grid - vec2<f32>(first);
        let layer = i32(record.track.x);
        let a = mix(textureLoad(bed_map, first, layer, 0),
            textureLoad(bed_map, first + CORNERS[1], layer, 0), uv.x);
        let b = mix(textureLoad(bed_map, first + CORNERS[2], layer, 0),
            textureLoad(bed_map, first + CORNERS[3], layer, 0), uv.x);
        let sample = mix(a, b, uv.y);
        // As a joined speed tends to zero, both departure reads meet the
        // receiving field continuously. Unjoined water keeps its reference.
        let scale = clamp(sample.zw * current / max(sample.zw * sample.zw, vec2(1e-12)),
            vec2(0.0), vec2(1.0));
        return vec4(sample.xy * scale, record.track.zw);
    }
    let axis_code = record.track.y;
    if axis_code == 0.0 || all(current == vec2(0.0)) { return vec4(0.0); }
    let axis = u32(axis_code) - 1u;
    let last = map.size_count_sea[axis] - 1u;
    let position = clamp((point[axis] - map.origin_spacing_margin[axis])
        / map.origin_spacing_margin.z, 0.0, f32(last));
    let cell = min(u32(position), last - 1u);
    var lower = vec2<i32>(0);
    lower[axis] = i32(cell);
    var upper = lower;
    upper[axis] += 1;
    let layer = i32(record.layers.z) + 2;
    let a = textureLoad(bed_map, lower, layer, 0).xy;
    let b = textureLoad(bed_map, upper, layer, 0).xy;
    let route = mix(a, b, position - f32(cell));
    let blend = clamp(abs(current[axis]) / route.y, 0.0, 1.0);
    return vec4(route.x, record.track.x, blend, axis_code);
}
