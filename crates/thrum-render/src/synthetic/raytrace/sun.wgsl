fn shadow_tangent_surface(model: mat4x4<f32>, local: vec3<f32>, normal: vec3<f32>) -> vec4<f32> {
    // Interpolate without the world translation so the tangent-plane gap is
    // still meaningful for small triangles on distant meshes.
    let relative = (model * vec4(local, 0.0)).xyz;
    return vec4(relative, dot(relative, normal));
}

fn shadow_hit(origin: vec3<f32>, direction: vec3<f32>, distance: f32, flags: u32) -> RayIntersection {
    var query: ray_query;
    rayQueryInitialize(&query, scene_structure, RayDesc(flags, 1u, 0.0, distance, origin, direction));
    while rayQueryProceed(&query) {}
    return rayQueryGetCommittedIntersection(&query);
}

fn sun_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    let direction = light.direction_to_light.xyz;
    let cosine = dot(receiver.normal, direction);
    if cosine <= 0.0 { return 1.0; }
    let origin = shadow_ray_origin(receiver.position, receiver.normal);
    let surface = input.shadow_surface;
    let gap = (surface.w - dot(surface.xyz, input.normal)) / max(length(input.normal), 1e-6);
    let lift = length(origin - receiver.position);
    if cosine >= 0.65 || gap <= lift {
        return select(1.0, 0.0, shadow_ray_blocked(origin, direction, 1e30));
    }

    // A coarse convex patch can intercept its own light between the
    // flat triangle and the surface implied by the interpolated normals.
    // Treat those nearby hits on the receiving instance as part of the
    // terminator. Every other caster sees the original ray.
    let escape = 4.0 * gap / max(cosine, 0.05);
    let blocker = shadow_hit(origin, direction, 1e30, RAY_FLAG_FORCE_OPAQUE | RAY_FLAG_TERMINATE_ON_FIRST_HIT);
    if blocker.kind == RAY_QUERY_INTERSECTION_NONE { return 1.0; }
    if blocker.t > escape { return 0.0; }

    // Look back across the numeric lift to identify the surface using
    // the same scene structure, including cooked placements.
    let owner = shadow_hit(origin, -receiver.normal, 2.0 * lift, RAY_FLAG_FORCE_OPAQUE);
    if owner.kind == RAY_QUERY_INTERSECTION_NONE || owner.instance_custom_data != blocker.instance_custom_data {
        return 0.0;
    }
    var query: ray_query;
    rayQueryInitialize(&query, scene_structure, RayDesc(RAY_FLAG_FORCE_NO_OPAQUE | RAY_FLAG_TERMINATE_ON_FIRST_HIT, 1u, 0.0, 1e30, origin, direction));
    while rayQueryProceed(&query) {
        let hit = rayQueryGetCandidateIntersection(&query);
        let receiving_triangle = hit.geometry_index == owner.geometry_index
            && hit.primitive_index == owner.primitive_index;
        let leaves_surface = hit.front_face != owner.front_face;
        if hit.instance_custom_data != owner.instance_custom_data || hit.t > escape
            || !(receiving_triangle || leaves_surface) {
            rayQueryConfirmIntersection(&query);
        }
    }
    return select(1.0, 0.0, rayQueryGetCommittedIntersection(&query).kind != RAY_QUERY_INTERSECTION_NONE);
}
