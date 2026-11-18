enable wgpu_ray_query;

@group(0) @binding(0) var scene: acceleration_structure;
@group(1) @binding(0) var<storage, read> origins: array<vec4<f32>>;
@group(1) @binding(1) var<storage, read_write> answers: array<u32>;

@compute @workgroup_size(64)
fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&origins) { return; }
    var query: ray_query;
    rayQueryInitialize(&query, scene, RayDesc(
        RAY_FLAG_TERMINATE_ON_FIRST_HIT, 0x1u, 0.001, 100.0, origins[id.x].xyz, vec3(0.0, 0.0, 1.0)));
    rayQueryProceed(&query);
    let hit = rayQueryGetCommittedIntersection(&query);
    answers[id.x] = select(0u, hit.instance_custom_data + 1u, hit.kind != RAY_QUERY_INTERSECTION_NONE);
}
