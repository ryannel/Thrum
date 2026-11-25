struct SurfVertexSample {
    current: vec4<f32>,
    previous: vec4<f32>,
    short_weights: vec2<f32>,
    unresolved_throw: vec2<u32>,
}
@group(3) @binding(13) var<storage, read> surf_cache: array<SurfVertexSample>;
fn visible_surf_sample(sample: vec4<f32>, packed_throw: u32) -> vec4<f32> {
    let shift = unpack2x16float(packed_throw);
    return sample - vec4(shift.x, 0.0, shift.y, 0.0);
}
fn surf_current(index: u32) -> vec4<f32> {
    let sample = surf_cache[index];
    return visible_surf_sample(sample.current, sample.unresolved_throw.x);
}
fn surf_previous(index: u32) -> vec4<f32> {
    let sample = surf_cache[index];
    return visible_surf_sample(sample.previous, sample.unresolved_throw.y);
}
fn surf_short_weights(index: u32) -> vec2<f32> { return surf_cache[index].short_weights; }

fn surf_shore_face(point: vec2<f32>) -> f32 {
    let end = arrayLength(&surf_cache) - 1u;
    let size = surf_cache[end].unresolved_throw;
    let span = vec2<f32>(map.size_count_sea.xy - 1u) * map.origin_spacing_margin.z;
    let uv = (point - map.origin_spacing_margin.xy) / span;
    if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) { return 0.0; }
    let grid = uv * vec2<f32>(size - 1u);
    let first = min(vec2<u32>(grid), size - 2u);
    let f = grid - vec2<f32>(first);
    let base = end - size.x * size.y + first.y * size.x + first.x;
    let a = surf_cache[base].current.xyz;
    let b = surf_cache[base + 1u].current.xyz;
    let c = surf_cache[base + size.x].current.xyz;
    let d = surf_cache[base + size.x + 1u].current.xyz;
    let wave = mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
    if wave.z < 1.0e-5 { return 0.0; }
    // Filter the crest phase before applying the face profile so the narrow
    // bright line can travel continuously between the bed lattice nodes.
    let phase = atan2(wave.y, wave.x);
    return wave.z * smoothstep(-0.20, 0.22, phase) * (1.0 - smoothstep(0.65, 1.12, phase));
}
