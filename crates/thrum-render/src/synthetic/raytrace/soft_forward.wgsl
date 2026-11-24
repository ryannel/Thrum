@group(1) @binding(19) var<uniform> soft_frame: SoftSunFrame;
@group(1) @binding(20) var soft_values: texture_2d<f32>;
@group(1) @binding(21) var soft_samples: texture_2d<u32>;

fn sun_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    if soft_frame.enabled == 0u { return hard_sun_visibility(input, receiver); }
    let direction = light.direction_to_light.xyz;
    let cosine = dot(receiver.normal, direction);
    if cosine <= 0.0 { return 1.0; }

    let plane = cross(receiver.dx, receiver.dy);
    let outward = select(-plane, plane, dot(plane, receiver.normal) >= 0.0);
    let band = dot(outward, direction) <= sin(soft_frame.radius) * length(plane);
    let origin = shadow_ray_origin(receiver.position, receiver.normal);
    var centre = 1.0;
    var granted = false;
    if band {
        let surface = input.shadow_surface;
        let gap = (surface.w - dot(surface.xyz, input.normal)) / max(length(input.normal), 1e-6);
        let inside = shadow_ray_origin(receiver.position, -receiver.normal);
        let crossing = closest_shadow_hit(inside, direction, 1e30);
        if crossing.kind != RAY_QUERY_INTERSECTION_NONE {
            let lift = dot(receiver.position - inside, receiver.normal);
            if gap > 0.0 && crossing.t * cosine <= 21.0 * gap + lift {
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
    return select(soft, centre, settled && centre != soft);
}
