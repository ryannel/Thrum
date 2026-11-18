@group(4) @binding(3) var<storage, read> mesh_attributes: array<u32>;

@task @payload(cooked_payload) @workgroup_size(1)
fn cooked_object(@builtin(workgroup_id) group: vec3<u32>) -> @builtin(mesh_task_size) vec3<u32> {
    var count = 0u;
    let begin = group.x * 64u;
    let end = begin + min(64u, mesh_placement.draw.y - begin);
    let camera_scale = vec4(view.camera_xyz_time.xyz, bitcast<f32>(mesh_placement.draw.w));
    for (var i = begin; i < end; i++) {
        var rejection = 0u;
        if mesh_placement.draw.z != 0u {
            rejection = meshlet_rejection(mesh_bounds[i], mesh_placement.model, view.view_proj, camera_scale);
        }
        cooked_report_rejection(rejection);
        if rejection == 0u {
            cooked_payload.survivors[count] = i;
            count++;
        }
    }
    return vec3(count, 1u, 1u);
}

struct CookedMeshOutput {
    @builtin(vertices) vertices: array<VertexOutput, 64>,
    @builtin(primitives) primitives: array<CookedPrimitive, 124>,
    @builtin(vertex_count) vertex_count: u32,
    @builtin(primitive_count) primitive_count: u32,
}
var<workgroup> cooked_output: CookedMeshOutput;

@mesh(cooked_output) @payload(cooked_payload) @workgroup_size(1)
fn cooked_mesh(@builtin(workgroup_id) group: vec3<u32>) {
    let meshlet = mesh_table[cooked_payload.survivors[group.x]];
    cooked_output.vertex_count = meshlet.vertex_count;
    cooked_output.primitive_count = meshlet.triangle_count;
    for (var i = 0u; i < meshlet.vertex_count; i++) {
        let vertex = cooked_meshlet_vertices[meshlet.vertex_offset + i] * 3u;
        let position = mesh_position(vertex);
        let normal = unpack2x16snorm(mesh_attributes[vertex]);
        let uv = bitcast<vec2<f32>>(vec2(mesh_attributes[vertex + 1u], mesh_attributes[vertex + 2u]));
        cooked_output.vertices[i] = cooked_vertex_at(position, normal, uv, mesh_placement.model, mesh_placement.draw.x);
    }
    for (var i = 0u; i < meshlet.triangle_count; i++) {
        cooked_output.primitives[i].indices = mesh_triangle(meshlet.triangle_offset + i * 3u);
    }
}
