// Test-only entry point appended to the production TAAU module.
@group(0) @binding(12) var<storage, read_write> probe_output: array<vec4<f32>, 10>;
@group(0) @binding(13) var<storage, read> probe_cases: array<vec4<f32>, 5>;

@compute @workgroup_size(1)
fn probe_depth(@builtin(global_invocation_id) id: vec3<u32>) {
    let row = probe_cases[id.x];
    let sample = depth_at(vec2(row.y,row.z+f32(id.x*2u)),row.x,textureDimensions(previous_depth));
    let sky = row.w != 0.0;
    let tolerance = max(0.01,row.x*0.01);
    let old_range_control = (sky && sample.y == 0.0)
        || (!sky && sample.x > 0.0 && row.x >= sample.x-tolerance && row.x <= sample.y+tolerance);
    probe_output[id.x*2u] = vec4(sample.xyz,depth_confidence(sample,row.x,sky));
    // An independent quadratic field exposes bilinear history blur. Evaluate
    // the production interpolation weights against its integer-grid samples;
    // the requested analytic value is x*x, while bilinear gives x.
    let x = row.y;
    var quadratic = 0.0;
    for (var k = -1; k <= 2; k++) {
        quadratic += f32(k*k)*history_filter(f32(k)-x);
    }
    probe_output[id.x*2u+1u] = vec4(select(0.0,1.0,old_range_control),quadratic,x*x,x);
}
