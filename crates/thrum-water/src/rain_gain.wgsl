struct RainSurface {
    lengths: vec4<f32>,
    canopy: vec4<f32>,
    grid: vec4<u32>,
    footprint_scale: vec4<f32>,
}
@group(3) @binding(19) var<uniform> rain: RainSurface;
@group(3) @binding(20) var rain_columns: texture_2d<f32>;

fn rain_exposure(point: vec2<f32>, level: f32) -> f32 {
    let code = rain.footprint_scale.z;
    var offset = vec2(0.0);
    if code != 0.0 { offset = canopy_load(rain_columns, 0u).xy; }
    let p = (point - rain.canopy.xy) / rain.canopy.z - offset;
    let half_width = 0.5 * rain.canopy.w / rain.canopy.z;
    if any(p < vec2(-half_width)) || any(p >= vec2<f32>(rain.grid.xy) + half_width) { return 1.0; }
    return canopy_exposure(rain_columns, vec3(point.x, level, point.y), rain.canopy,
        vec2<i32>(rain.grid.xy), select(0u, 1u, code != 0.0), code, vec2<i32>(offset));
}

fn rain_response(index: u32) -> vec2<f32> {
    return canopy_load(rain_columns, u32(rain.footprint_scale.w) + index).xy;
}

fn rain_gain(point: vec2<f32>, row: SurfaceRow) -> f32 {
    let depth = row.level - row.bed;
    if rain.grid.w == 0u || depth <= 0.0 { return 0.0; }
    let deepest = rain_response(rain.grid.z - 1u);
    var response = deepest.y;
    var previous = rain_response(0u);
    for (var i = 1u; depth < deepest.x && i < rain.grid.z; i += 1u) {
        let next = rain_response(i);
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
