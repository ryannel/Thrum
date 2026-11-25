struct SurfVertexSample { current: vec4<f32>, previous: vec4<f32>, short_weights: vec4<f32>, }
@group(3) @binding(13) var<storage, read> surf_cache: array<SurfVertexSample>;
fn surf_current(index: u32) -> vec4<f32> { return surf_cache[index].current; }
fn surf_previous(index: u32) -> vec4<f32> { return surf_cache[index].previous; }
fn surf_short_weights(index: u32) -> vec2<f32> { return surf_cache[index].short_weights.xy; }

fn surf_shore_face(point: vec2<f32>) -> f32 {
    let end = arrayLength(&surf_cache) - 1u;
    let size = vec2<u32>(surf_cache[end].short_weights.zw);
    let span = vec2<f32>(map.size_count_sea.xy - 1u) * map.origin_spacing_margin.z;
    let uv = (point - map.origin_spacing_margin.xy) / span;
    if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) { return 0.0; }
    let grid = uv * vec2<f32>(size - 1u);
    let first = min(vec2<u32>(grid), size - 2u);
    let f = grid - vec2<f32>(first);
    let base = end - size.x * size.y + first.y * size.x + first.x;
    let a = surf_cache[base].current.x;
    let b = surf_cache[base + 1u].current.x;
    let c = surf_cache[base + size.x].current.x;
    let d = surf_cache[base + size.x + 1u].current.x;
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
