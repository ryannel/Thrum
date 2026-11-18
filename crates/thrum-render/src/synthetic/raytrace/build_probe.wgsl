enable wgpu_ray_query;

@group(0) @binding(0) var<storage, read_write> sink: atomic<u32>;
@group(1) @binding(0) var structure: acceleration_structure;

@compute @workgroup_size(1)
fn mark() {
    atomicAdd(&sink, 1u);
}

@compute @workgroup_size(1)
fn consume() {
    var query: ray_query;
    rayQueryInitialize(&query, structure, RayDesc(
        RAY_FLAG_TERMINATE_ON_FIRST_HIT, 1u, 0.001, 20000.0,
        vec3(0.01, 10000.0, 0.013), vec3(0.0, -1.0, 0.0)));
    rayQueryProceed(&query);
    atomicAdd(&sink, rayQueryGetCommittedIntersection(&query).kind);
}
