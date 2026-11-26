
struct HollowsRectangle {
    origin_spacing: vec4<f32>,
    size_offsets: vec4<u32>,
    optics_offset: vec4<u32>,
}
struct HollowsField {
    rectangles: array<HollowsRectangle, BODY_LIMIT>,
    cells: array<vec2<u32>>, // bed and label, current and previous levels, then optional optical types
}
@group(WATER_MAP_GROUP) @binding(18) var<storage, read> hollows: HollowsField;

fn hollows_cell_at(rectangle: HollowsRectangle, point: vec2<f32>) -> vec2<i32> {
    return vec2<i32>(floor((point - rectangle.origin_spacing.xy) / rectangle.origin_spacing.z));
}

fn hollows_optical_type(body: u32, point: vec2<f32>) -> vec4<f32> {
    let rectangle = hollows.rectangles[body];
    let base = rectangle.optics_offset.x;
    let at = hollows_cell_at(rectangle, point);
    if base == NO_BODY || any(at < vec2(0)) || any(at >= vec2<i32>(rectangle.size_offsets.xy)) {
        return vec4(0.0);
    }
    let index = rectangle.size_offsets.z + u32(at.y)*rectangle.size_offsets.x + u32(at.x);
    let label = hollows.cells[index].y;
    if label == NO_BODY { return vec4(0.0); }
    let record = base + 2u*label;
    return bitcast<vec4<f32>>(vec4(hollows.cells[record], hollows.cells[record + 1u]));
}

fn hollows_wet(rectangle: HollowsRectangle, at: vec2<i32>) -> vec2<f32> {
    if any(at < vec2(0)) || any(at >= vec2<i32>(rectangle.size_offsets.xy)) {
        return vec2(-1e30);
    }
    let index = rectangle.size_offsets.z + u32(at.y)*rectangle.size_offsets.x + u32(at.x);
    let cell = hollows.cells[index];
    if cell.y == NO_BODY { return vec2(-1e30); }
    let levels = bitcast<vec2<f32>>(hollows.cells[rectangle.size_offsets.w + cell.y]);
    return select(vec2(-1e30), levels, levels.x > bitcast<f32>(cell.x));
}

fn hollows_surface(rectangle: HollowsRectangle, at: vec2<i32>) -> vec2<f32> {
    var levels = hollows_wet(rectangle, at);
    if levels.x > -1e30 { return levels; }
    for (var z = -1; z <= 1; z++) {
        for (var x = -1; x <= 1; x++) {
            let candidate = hollows_wet(rectangle, at + vec2(x, z));
            if candidate.x > levels.x { levels = candidate; }
        }
    }
    return levels;
}

fn hollows_map_sample(point: vec2<f32>) -> MapAnswer {
    for (var id = 0u; id < map.size_count_sea.z; id++) {
        if map.bodies[id].layers.x != 3u { continue; }
        let rectangle = hollows.rectangles[id];
        let at = hollows_cell_at(rectangle, point);
        if any(at < vec2(0)) || any(at >= vec2<i32>(rectangle.size_offsets.xy)) {
            continue;
        }
        let index = rectangle.size_offsets.z + u32(at.y)*rectangle.size_offsets.x + u32(at.x);
        let bed = bitcast<f32>(hollows.cells[index].x);
        let level = hollows_surface(rectangle, at).x;
        if level <= -1e30 {
            return MapAnswer(NO_BODY, MapRow(vec3(0.0), vec4(0.0)), bed, true);
        }
        return MapAnswer(id, MapRow(vec3(level, 0.0, 0.0), vec4(0.0)), bed, true);
    }
    return map_sample(point);
}
