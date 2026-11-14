// Test-only entry point appended to the production TAAU module.
@group(0) @binding(12) var<storage, read_write> probe_output: array<vec4<f32>, 8>;
@group(0) @binding(13) var<storage, read> probe_cases: array<vec4<f32>, 4>;

@compute @workgroup_size(4)
fn probe_depth(@builtin(global_invocation_id) id: vec3<u32>) {
    let row = probe_cases[id.x];
    let sample = depth_at(vec2(row.y,row.z+f32(id.x*2u)));
    let sky = row.w != 0.0;
    let tolerance = max(0.01,row.x*0.01);
    let old_range_control = (sky && sample.y == 0.0)
        || (!sky && sample.x > 0.0 && row.x >= sample.x-tolerance && row.x <= sample.y+tolerance);
    probe_output[id.x*2u] = vec4(sample,select(0.0,1.0,depth_matches(sample,row.x,sky)));
    probe_output[id.x*2u+1u] = vec4(select(0.0,1.0,old_range_control),row.x,0.0,0.0);
}
