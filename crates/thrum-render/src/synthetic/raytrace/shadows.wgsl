@group(1) @binding(15) var scene_structure: acceleration_structure;

fn shadow_ray_origin(point: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    return shadow_ray_origin_at_distance(point, normal, 0.0);
}
fn shadow_ray_origin_at_distance(point: vec3<f32>, normal: vec3<f32>, distance: f32) -> vec3<f32> {
    let magnitude = max(max(max(abs(point.x), abs(point.y)), abs(point.z)), max(distance, 1000.0));
    let spacing = bitcast<f32>(bitcast<u32>(magnitude) + 1u) - magnitude;
    return point + normal * (8.0 * spacing);
}

fn shadow_ray_blocked(origin: vec3<f32>, direction: vec3<f32>, distance: f32) -> bool {
    return shadow_ray_blocked_from(origin, direction, 0.0, distance);
}

fn shadow_ray_blocked_from(origin: vec3<f32>, direction: vec3<f32>, start: f32, distance: f32) -> bool {
    var query: ray_query;
    let flags = RAY_FLAG_FORCE_OPAQUE | RAY_FLAG_TERMINATE_ON_FIRST_HIT;
    rayQueryInitialize(&query, scene_structure, RayDesc(flags, 1u, start, distance, origin, direction));
    while rayQueryProceed(&query) {}
    return rayQueryGetCommittedIntersection(&query).kind != RAY_QUERY_INTERSECTION_NONE;
}

fn closest_shadow_hit(origin: vec3<f32>, direction: vec3<f32>, distance: f32) -> RayIntersection {
    return closest_shadow_hit_from(origin, direction, 0.0, distance);
}

fn closest_shadow_hit_from(origin: vec3<f32>, direction: vec3<f32>, start: f32, distance: f32) -> RayIntersection {
    var query: ray_query;
    rayQueryInitialize(&query, scene_structure, RayDesc(RAY_FLAG_FORCE_OPAQUE, 1u, start, distance, origin, direction));
    while rayQueryProceed(&query) {}
    return rayQueryGetCommittedIntersection(&query);
}

fn shadow_same_receiver(origin: vec3<f32>, inside: vec3<f32>, normal: vec3<f32>, crossing: RayIntersection) -> bool {
    let owner = closest_shadow_hit(origin, -normal, length(origin - inside));
    return owner.kind != RAY_QUERY_INTERSECTION_NONE
        && owner.instance_custom_data == crossing.instance_custom_data
        && owner.front_face != crossing.front_face;
}

fn shadow_after(crossing: RayIntersection) -> f32 {
    return bitcast<f32>(bitcast<u32>(crossing.t) + 1u);
}
