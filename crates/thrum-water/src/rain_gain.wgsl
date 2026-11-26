struct RainSurface {
    lengths: vec4<f32>,
    canopy: vec4<f32>,
    grid: vec4<u32>,
}
@group(3) @binding(19) var<uniform> rain: RainSurface;
@group(3) @binding(20) var<storage, read> rain_columns: array<vec4<f32>>;

fn rain_column(cell: vec2<i32>, level: f32) -> f32 {
    if any(cell < vec2(0)) || any(vec2<u32>(cell) >= rain.grid.xy) { return 1.0; }
    let column = rain_columns[rain.grid.z + u32(cell.y) * rain.grid.x + u32(cell.x)];
    return select(1.0, column.y, level < column.x) * select(1.0, column.w, level < column.z);
}

fn rain_exposure(point: vec2<f32>, level: f32) -> f32 {
    let p = (point - rain.canopy.xy) / rain.canopy.z;
    let half_width = 0.5 * rain.canopy.w / rain.canopy.z;
    if any(p < vec2(-half_width)) || any(p >= vec2<f32>(rain.grid.xy) + half_width) { return 1.0; }
    if half_width == 0.0 { return rain_column(vec2<i32>(floor(p)), level); }
    let lo = p - half_width;
    let hi = p + half_width;
    let first = vec2<i32>(floor(lo));
    let last = vec2<i32>(floor(hi));
    var sum = 0.0;
    for (var z = first.y; z <= last.y; z += 1) {
        for (var x = first.x; x <= last.x; x += 1) {
            let cell = vec2(x, z);
            let area = max(vec2(0.0), min(vec2<f32>(cell)+1.0, hi) - max(vec2<f32>(cell), lo));
            sum += area.x * area.y * rain_column(cell, level);
        }
    }
    return min(1.0, sum / (4.0 * half_width * half_width));
}

fn rain_gain(point: vec2<f32>, row: SurfaceRow) -> f32 {
    let depth = row.level - row.bed;
    if rain.grid.w == 0u || depth <= 0.0 { return 0.0; }
    let deepest = rain_columns[rain.grid.z - 1u].xy;
    var response = deepest.y;
    var previous = rain_columns[0].xy;
    for (var i = 1u; depth < deepest.x && i < rain.grid.z; i += 1u) {
        let next = rain_columns[i].xy;
        if depth < next.x {
            response = mix(previous.y, next.y, clamp((depth-previous.x)/(next.x-previous.x), 0.0, 1.0));
            break;
        }
        previous = next;
        response = next.y;
    }
    if row.bed < -1e29 { response = 1.0; }
    let maximum = 0.6 * (depth + row.swash) / max(rain.lengths.w, 1e-30);
    return min(response * rain_exposure(point, row.level), maximum);
}
