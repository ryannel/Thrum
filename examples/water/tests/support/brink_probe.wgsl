@group(0) @binding(0) var<storage, read> points: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> answers: array<vec4<f32>>;

@compute @workgroup_size(32) fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&points) { return; }
    let answer = map_sample(points[id.x]);
    let body = select(-1.0, f32(answer.body), answer.body != NO_BODY);
    answers[id.x] = vec4(body, answer.row.water.x, answer.bed, answer.row.water.x - answer.bed);
}
