struct SunProbePoint {
    position: vec4<f32>,
    normal: vec4<f32>,
}
@group(2) @binding(0) var<storage, read> sample_points: array<SunProbePoint, 2>;
@group(2) @binding(1) var<storage, read_write> visibility_values: array<f32, 2>;

@compute @workgroup_size(1)
fn sample_sun(@builtin(global_invocation_id) id: vec3<u32>) {
    var vertex_output: VertexOutput;
    let sample = sample_points[id.x];
    let receiver = ShadowReceiver(sample.position.xyz, vec3(0.0), vec3(0.0), sample.normal.xyz);
    visibility_values[id.x] = sun_visibility(vertex_output, receiver);
}
