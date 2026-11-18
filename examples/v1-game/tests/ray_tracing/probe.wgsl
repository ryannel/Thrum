enable wgpu_ray_query;

struct Ray {
    origin_and_mask: vec4<f32>,
    direction_and_distance: vec4<f32>,
}

@group(0) @binding(0) var scene: acceleration_structure;
@group(1) @binding(0) var<storage, read> rays: array<Ray>;
@group(1) @binding(1) var<storage, read_write> answers: array<u32>;

@compute @workgroup_size(64)
fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&rays) { return; }
    let ray = rays[id.x];
    var query: ray_query;
    rayQueryInitialize(&query, scene, RayDesc(
        0u, u32(ray.origin_and_mask.w), 0.0, ray.direction_and_distance.w,
        ray.origin_and_mask.xyz, ray.direction_and_distance.xyz));
    rayQueryProceed(&query);
    let hit = rayQueryGetCommittedIntersection(&query);
    answers[id.x] = select(0u, hit.instance_custom_data + 1u, hit.kind != RAY_QUERY_INTERSECTION_NONE);
}
