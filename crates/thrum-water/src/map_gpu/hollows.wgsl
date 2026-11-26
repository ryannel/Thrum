
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

struct HollowOpticalType {
    sediment: vec4<f32>,
    bed_tint: vec3<f32>,
}

fn hollows_optical_type(body: u32, point: vec2<f32>) -> HollowOpticalType {
    let rectangle = hollows.rectangles[body];
    let base = rectangle.optics_offset.x;
    let label = hollows_surface(rectangle, hollows_cell_at(rectangle, point)).label;
    if base == NO_BODY || label == NO_BODY {
        return HollowOpticalType(vec4(0.0), vec3(1.0));
    }
    let record = base + 4u*label;
    let sediment = bitcast<vec4<f32>>(vec4(hollows.cells[record], hollows.cells[record + 1u]));
    let tint = bitcast<vec4<f32>>(vec4(hollows.cells[record + 2u], hollows.cells[record + 3u]));
    return HollowOpticalType(sediment, tint.xyz);
}

struct HollowSurface {
    levels: vec2<f32>,
    label: u32,
}

fn hollows_wet(rectangle: HollowsRectangle, at: vec2<i32>) -> HollowSurface {
    let dry = HollowSurface(vec2(-1e30), NO_BODY);
    if any(at < vec2(0)) || any(at >= vec2<i32>(rectangle.size_offsets.xy)) { return dry; }
    let index = rectangle.size_offsets.z + u32(at.y)*rectangle.size_offsets.x + u32(at.x);
    let cell = hollows.cells[index];
    if cell.y == NO_BODY { return dry; }
    let levels = bitcast<vec2<f32>>(hollows.cells[rectangle.size_offsets.w + cell.y]);
    if levels.x <= bitcast<f32>(cell.x) { return dry; }
    return HollowSurface(levels, cell.y);
}

fn hollows_surface(rectangle: HollowsRectangle, at: vec2<i32>) -> HollowSurface {
    var sample = hollows_wet(rectangle, at);
    if sample.label != NO_BODY { return sample; }
    for (var z = -1; z <= 1; z++) {
        for (var x = -1; x <= 1; x++) {
            let candidate = hollows_wet(rectangle, at + vec2(x, z));
            if candidate.levels.x > sample.levels.x { sample = candidate; }
        }
    }
    return sample;
}

fn hollows_sample(point: vec2<f32>) -> MapAnswer {
    for (var id = 0u; id < map.size_count_sea.z; id++) {
        if map.bodies[id].layers.x != 3u { continue; }
        let rectangle = hollows.rectangles[id];
        let at = hollows_cell_at(rectangle, point);
        if any(at < vec2(0)) || any(at >= vec2<i32>(rectangle.size_offsets.xy)) {
            continue;
        }
        let index = rectangle.size_offsets.z + u32(at.y)*rectangle.size_offsets.x + u32(at.x);
        let bed = bitcast<f32>(hollows.cells[index].x);
        let level = hollows_surface(rectangle, at).levels.x;
        if level <= -1e30 {
            return MapAnswer(NO_BODY, MapRow(vec3(0.0), vec4(0.0)), bed, true);
        }
        return MapAnswer(id, MapRow(vec3(level, 0.0, 0.0), vec4(0.0)), bed, true);
    }
    return MapAnswer(NO_BODY, MapRow(vec3(0.0), vec4(0.0)), 0.0, false);
}
