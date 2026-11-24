@group(1) @binding(19) var<uniform> soft_frame: SoftSunFrame;
@group(1) @binding(20) var soft_values: texture_2d<f32>;
@group(1) @binding(21) var soft_samples: texture_2d<u32>;

fn verify_sun_rim(origin: vec3<f32>, inside: vec3<f32>, normal: vec3<f32>,
    outward: vec3<f32>, direction: vec3<f32>, centre: f32, granted: bool) -> bool {
    let tangent = normalize(cross(direction, select(vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0), abs(direction.y) > 0.9)));
    let bitangent = cross(direction, tangent);
    let rim = array(tangent, -tangent, bitangent, -bitangent);
    let radius = tan(soft_frame.radius);
    for (var sample = 0u; sample < soft_frame.rim_rays; sample++) {
        let ray = normalize(direction + rim[sample] * radius);
        var blocked: bool;
        if dot(outward, ray) >= 0.0 {
            blocked = shadow_ray_blocked(origin, ray, 1e30);
        } else if granted {
            let crossing = closest_shadow_hit(inside, ray, 1e30);
            if crossing.kind == RAY_QUERY_INTERSECTION_NONE {
                blocked = false;
            } else if shadow_same_receiver(origin, inside, normal, crossing) {
                blocked = shadow_ray_blocked_from(inside, ray, shadow_after(crossing), 1e30);
            } else {
                blocked = true;
            }
        } else {
            blocked = shadow_ray_blocked(inside, ray, 1e30);
        }
        if blocked != (centre == 0.0) { return false; }
    }
    return true;
}

fn sun_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    if soft_frame.enabled == 0u { return hard_sun_visibility(input, receiver); }
    let direction = light.direction_to_light.xyz;
    let cosine = dot(receiver.normal, direction);
    // Bound the local body passage by the scale implied by the change in
    // shading normal. This keeps a nearly tangent ray from excusing a remote
    // sheet joined to the receiving mesh while preserving the gap test.
    let normal_change = max(length(dpdx(receiver.normal)), length(dpdy(receiver.normal)));
    let footprint = max(length(receiver.dx), length(receiver.dy));
    let passage_limit = 4.0 * footprint / max(normal_change, 1e-8);
    if cosine <= 0.0 { return 1.0; }

    let plane = cross(receiver.dx, receiver.dy);
    let outward = select(-plane, plane, dot(plane, receiver.normal) >= 0.0);
    let plane_cosine = dot(outward, direction) / max(length(plane), 1e-12);
    let band = plane_cosine <= sin(soft_frame.radius);
    let origin = shadow_ray_origin(receiver.position, receiver.normal);
    let inside = shadow_ray_origin(receiver.position, -receiver.normal);
    var centre = 1.0;
    var granted = false;
    if band {
        let surface = input.shadow_surface;
        let gap = (surface.w - dot(surface.xyz, input.normal)) / max(length(input.normal), 1e-6);
        let crossing = closest_shadow_hit(inside, direction, 1e30);
        if crossing.kind != RAY_QUERY_INTERSECTION_NONE {
            let lift = dot(receiver.position - inside, receiver.normal);
            // A vanishing shading cosine cannot make the reach unbounded while
            // the ray still descends through this face's plane.
            let reach_cosine = max(cosine, -plane_cosine);
            if gap > 0.0 && crossing.t <= passage_limit && crossing.t * reach_cosine <= 21.0 * gap + lift {
                granted = shadow_same_receiver(origin, inside, receiver.normal, crossing);
            }
            centre = select(0.0, 1.0, granted &&
                !shadow_ray_blocked_from(inside, direction, shadow_after(crossing), 1e30));
        }
    } else {
        centre = select(1.0, 0.0, shadow_ray_blocked(origin, direction, 1e30));
    }

    let previous = input.current_clip + input.clip_delta;
    if soft_frame.valid == 0u || input.history_valid == 0u || previous.w <= 0.0 {
        return centre;
    }
    let pixel = vec2<i32>(floor(input.position.xy - motion_vector(input) + soft_frame.jitter_delta));
    if !soft_within(pixel, soft_frame.extent) { return centre; }
    let samples = textureLoad(soft_samples, pixel, 0);
    if !soft_depth_matches(bitcast<f32>(samples.z), previous.z / previous.w) {
        return centre;
    }
    let values = textureLoad(soft_values, pixel, 0);
    let channel = select(0u, 1u, band && !granted);
    let soft = values[channel];
    let settled = values[channel + 2u] > 0.0;
    if settled { return centre; }
    // The stored value may round to an endpoint even though a ray disagreed.
    // Retain that cue. Recheck fractional edges against the disc right now
    // so an edge that moved away cannot stay attached to the old silhouette.
    if soft_frame.rim_rays > 0u && soft > 0.0 && soft < 1.0 &&
        verify_sun_rim(origin, inside, receiver.normal, outward, direction, centre, granted) {
        return centre;
    }
    return soft;
}
