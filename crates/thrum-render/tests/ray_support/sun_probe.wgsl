@group(2) @binding(0) var<storage, read> sample_points: array<vec4<f32>, 2>;
@group(2) @binding(1) var<storage, read_write> visibility_values: array<f32, 2>;

@compute @workgroup_size(1)
fn sample_sun(@builtin(global_invocation_id) id: vec3<u32>) {
    var vertex_output: VertexOutput;
    let point = sample_points[id.x].xyz;
    let receiver = ShadowReceiver(point, vec3(0.0), vec3(0.0), vec3(0.0, 1.0, 0.0));
    visibility_values[id.x] = sun_visibility(vertex_output, receiver);
}
