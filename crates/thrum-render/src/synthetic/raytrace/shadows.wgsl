@group(1) @binding(15) var scene_structure: acceleration_structure;

fn shadow_ray_origin(point: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let magnitude = max(max(max(abs(point.x), abs(point.y)), abs(point.z)), 1000.0);
    let spacing = bitcast<f32>(bitcast<u32>(magnitude) + 1u) - magnitude;
    return point + normal * (8.0 * spacing);
}

fn shadow_ray_blocked(origin: vec3<f32>, direction: vec3<f32>, distance: f32) -> bool {
    var query: ray_query;
    let flags = RAY_FLAG_FORCE_OPAQUE | RAY_FLAG_TERMINATE_ON_FIRST_HIT;
    rayQueryInitialize(&query, scene_structure, RayDesc(flags, 1u, 0.0, distance, origin, direction));
    while rayQueryProceed(&query) {}
    return rayQueryGetCommittedIntersection(&query).kind != RAY_QUERY_INTERSECTION_NONE;
}
