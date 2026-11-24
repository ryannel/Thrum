override rounds: u32;

@group(0) @binding(0) var<storage, read_write> scratch: array<u32>;
var<workgroup> samples: array<u32, 64>;

@compute @workgroup_size(8, 8)
fn load(
    @builtin(workgroup_id) group: vec3<u32>,
    @builtin(num_workgroups) groups: vec3<u32>,
    @builtin(local_invocation_index) lane: u32,
) {
    let index = group.x + group.y * groups.x;
    var value = f32((index * 64u + lane) % 1021u) * 0.001 + 0.01;
    for (var i = 0u; i < rounds; i += 1u) {
        value = fract(sin(value + f32(i) * 0.015625) * 997.0);
    }
    samples[lane] = bitcast<u32>(value);
    workgroupBarrier();
    if lane == 0u {
        var result = 0u;
        for (var i = 0u; i < 64u; i += 1u) {
            result ^= samples[i];
        }
        scratch[index] ^= result;
    }
}
