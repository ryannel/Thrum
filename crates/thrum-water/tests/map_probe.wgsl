struct Probe {
    point: vec2<f32>,
    replace_gains: f32,
    padding: f32,
    heights: vec4<f32>,
}
struct Result {
    map: vec4<f32>,
    row: vec4<f32>,
    heights: vec4<f32>,
}
@group(0) @binding(0) var<storage, read> probes: array<Probe>;
@group(0) @binding(1) var<storage, read_write> results: array<Result>;

@compute @workgroup_size(64) fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= arrayLength(&probes) { return; }
    let p = probes[i];
    let answer = map_sample(p.point);
    let depth = select(1e30, answer.row.water.x - answer.bed, answer.has_bed);
    let body = select(-1.0, f32(answer.body), answer.body != NO_BODY);
    let gains = select(answer.row.gains, vec4(1.0), p.replace_gains > 0.0);
    let swash = map.swash.x;
    let margin = map.origin_spacing_margin.w;
    results[i] = Result(
        vec4(body, answer.row.water.x, answer.bed, depth),
        vec4(answer.row.water.yz, f32(answer.has_bed), 0.0),
        limit_gains(gains, depth, p.heights, swash, margin) * p.heights,
    );
}
