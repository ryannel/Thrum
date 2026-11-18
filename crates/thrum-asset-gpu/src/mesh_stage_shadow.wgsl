@task @payload(cooked_payload) @workgroup_size(1)
fn shadow_object(@builtin(workgroup_id) group: vec3<u32>) -> @builtin(mesh_task_size) vec3<u32> {
    var count = 0u;
    let begin = group.x * 64u;
    let end = begin + min(64u, mesh_placement.draw.y - begin);
    let scale = bitcast<f32>(mesh_placement.draw.w);
    for (var i = begin; i < end; i++) {
        var rejection = 0u;
        if mesh_placement.draw.z != 0u {
            rejection = meshlet_frustum_rejection(mesh_bounds[i], mesh_placement.model, light_view_projection, scale);
        }
        cooked_report_rejection(rejection);
        if rejection == 0u {
            cooked_payload.survivors[count] = i;
            count++;
        }
    }
    return vec3(count, 1u, 1u);
}

struct ShadowVertex { @builtin(position) position: vec4<f32> }
struct ShadowOutput {
    @builtin(vertices) vertices: array<ShadowVertex, 64>,
    @builtin(primitives) primitives: array<CookedPrimitive, 124>,
    @builtin(vertex_count) vertex_count: u32,
    @builtin(primitive_count) primitive_count: u32,
}
var<workgroup> shadow_output: ShadowOutput;

@mesh(shadow_output) @payload(cooked_payload) @workgroup_size(1)
fn shadow_mesh(@builtin(workgroup_id) group: vec3<u32>) {
    let meshlet = mesh_table[cooked_payload.survivors[group.x]];
    shadow_output.vertex_count = meshlet.vertex_count;
    shadow_output.primitive_count = meshlet.triangle_count;
    for (var i = 0u; i < meshlet.vertex_count; i++) {
        let vertex = cooked_meshlet_vertices[meshlet.vertex_offset + i] * 3u;
        shadow_output.vertices[i].position = caster_position(mesh_position(vertex), mesh_placement.model);
    }
    for (var i = 0u; i < meshlet.triangle_count; i++) {
        shadow_output.primitives[i].indices = mesh_triangle(meshlet.triangle_offset + i * 3u);
    }
}
