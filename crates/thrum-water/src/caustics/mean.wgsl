// A concentrated landing triangle can miss all sample positions. Normalize
// each rung by its rasterized mean before filtering to preserve incident energy.
@group(0) @binding(0) var rasterized: texture_2d_array<f32>;
@group(0) @binding(1) var<storage, read_write> partial: array<f32>;
@group(0) @binding(2) var<storage, read_write> rung_mean: array<f32>;

var<workgroup> sums: array<f32, 256>;

fn group_sum(lane: u32, value: f32) -> f32 {
    sums[lane] = value;
    workgroupBarrier();
    for (var width = 128u; width > 0u; width /= 2u) {
        if lane < width { sums[lane] += sums[lane + width]; }
        workgroupBarrier();
    }
    return sums[0];
}

@compute @workgroup_size(16, 16, 1)
fn sum_tiles(
    @builtin(local_invocation_index) lane: u32,
    @builtin(global_invocation_id) pixel: vec3<u32>,
    @builtin(workgroup_id) tile: vec3<u32>,
) {
    let sample = textureLoad(rasterized, vec2<i32>(pixel.xy), i32(tile.z), 0).r;
    let mean = group_sum(lane, sample) / 256.0;
    if lane == 0u { partial[(tile.z * 32u + tile.y) * 32u + tile.x] = mean; }
}

@compute @workgroup_size(256, 1, 1)
fn sum_rungs(
    @builtin(local_invocation_index) lane: u32,
    @builtin(workgroup_id) rung: vec3<u32>,
) {
    let origin = rung.z * 1024u + lane;
    var value = 0.0;
    for (var chunk = 0u; chunk < 4u; chunk++) {
        value += partial[origin + chunk * 256u];
    }
    let mean = group_sum(lane, value) / 1024.0;
    if lane == 0u { rung_mean[rung.z] = mean; }
}
