fn shadow_tangent_surface(model: mat4x4<f32>, local: vec3<f32>, normal: vec3<f32>) -> vec4<f32> {
    // Interpolate without the world translation so the tangent-plane gap is
    // still meaningful for small triangles on distant meshes.
    let relative = (model * vec4(local, 0.0)).xyz;
    return vec4(relative, dot(relative, normal));
}

fn closest_shadow_hit(origin: vec3<f32>, direction: vec3<f32>, distance: f32) -> RayIntersection {
    var query: ray_query;
    rayQueryInitialize(&query, scene_structure, RayDesc(RAY_FLAG_FORCE_OPAQUE, 1u, 0.0, distance, origin, direction));
    while rayQueryProceed(&query) {}
    return rayQueryGetCommittedIntersection(&query);
}

fn sun_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    let direction = light.direction_to_light.xyz;
    let cosine = dot(receiver.normal, direction);
    if cosine <= 0.0 { return 1.0; }
    let origin = shadow_ray_origin(receiver.position, receiver.normal);
    // The plane is translation-free and faces the same side as the authored
    // normal, including when an instance reverses the raster winding.
    let plane = cross(receiver.dx, receiver.dy);
    let outward = select(-plane, plane, dot(plane, receiver.normal) >= 0.0);
    if dot(outward, direction) > 1e-4 * length(plane) {
        return select(1.0, 0.0, shadow_ray_blocked(origin, direction, 1e30));
    }

    let surface = input.shadow_surface;
    let gap = (surface.w - dot(surface.xyz, input.normal)) / max(length(input.normal), 1e-6);
    // Flat receivers (including ordinary point probes) have no implied shell.
    if gap <= 0.0 {
        return select(1.0, 0.0, shadow_ray_blocked(origin, direction, 1e30));
    }

    let inside = shadow_ray_origin(receiver.position, -receiver.normal);
    let crossing = closest_shadow_hit(inside, direction, 1e30);
    if crossing.kind == RAY_QUERY_INTERSECTION_NONE { return 1.0; }
    let lift = dot(receiver.position - inside, receiver.normal);
    // On the irregular rock, a small gap near an edge can precede a much
    // longer diagonal through the mesh. The fixed noon route needs up to
    // 20 times the tangent-plane gap there. Keep the origin lift unchanged.
    if crossing.t * cosine > 21.0 * gap + lift { return 0.0; }

    let owner = closest_shadow_hit(origin, -receiver.normal, length(origin - inside));
    if owner.kind == RAY_QUERY_INTERSECTION_NONE
        || owner.instance_custom_data != crossing.instance_custom_data
        || owner.front_face == crossing.front_face {
        return 0.0;
    }

    // Keep the same origin and direction. Advance the lower bound past
    // exactly one crossing, so even a second back face of this mesh blocks.
    let after_crossing = bitcast<f32>(bitcast<u32>(crossing.t) + 1u);
    return select(1.0, 0.0, shadow_ray_blocked_from(inside, direction, after_crossing, 1e30));
}
