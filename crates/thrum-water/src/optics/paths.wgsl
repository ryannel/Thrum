const OPTICS_ENABLED: bool = true;
@group(1) @binding(3) var gathered_radiance: texture_2d<f32>;
@group(1) @binding(4) var gathered_distance: texture_2d<f32>;

struct OpticalPaths {
    reflected: vec3<f32>,
    behind: vec3<f32>,
    transmission: vec3<f32>,
    thickness: f32,
}

struct GatherHit {
    point: vec3<f32>,
    distance: f32,
    pixel: vec2<u32>,
    edge: f32,
}

fn empty_hit() -> GatherHit {
    return GatherHit(vec3(0.0), 0.0, vec2(0u), 0.0);
}

fn gather_at(uv: vec2<f32>) -> GatherHit {
    if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) { return empty_hit(); }
    let pixel = vec2<u32>(uv * view_aux.viewport_inv_viewport.xy);
    if (optics_mask(pixel) & GATHER_SOURCE) == 0u { return empty_hit(); }
    let distance = textureLoad(gathered_distance, pixel, 0).x;
    if distance <= 0.0 { return empty_hit(); }
    let near_point = view_aux.inverse_view_proj * optics_ndc(optics_uv(pixel), 1.0);
    let camera = view.camera_xyz_time.xyz;
    let dir = normalize(near_point.xyz - camera * near_point.w);
    let sides = min(uv, 1.0 - uv);
    let edge = smoothstep(0.0, 0.05, min(sides.x, sides.y));
    return GatherHit(camera + dir * distance, distance, pixel, edge);
}

fn hit_light(hit: GatherHit) -> vec3<f32> {
    return textureLoad(gathered_radiance, hit.pixel, 0).rgb / environment_storage_scale();
}

// Slab interval in metres along the surface ray. Handle parallel axes without 0/0.
fn box_interval(origin: vec3<f32>, ray: vec3<f32>, body: OpticsBody, reach: f32) -> vec2<f32> {
    var result = vec2(0.015, reach);
    for (var axis = 0u; axis < 3u; axis++) {
        let d = ray[axis];
        let low = body.low[axis] - origin[axis];
        let high = body.high[axis] - origin[axis];
        if abs(d) < 0.000001 {
            if low > 0.0 || high < 0.0 { return vec2(1.0, 0.0); }
        } else {
            let t = vec2(low, high) / d;
            result = vec2(max(result.x, min(t.x, t.y)), min(result.y, max(t.x, t.y)));
        }
    }
    return result;
}

fn in_box(point: vec3<f32>, body: OpticsBody) -> bool {
    return all(point >= body.low.xyz - 0.025) && all(point <= body.high.xyz + 0.025);
}

// The opaque depth at a hit must be near the ray itself. A foreground
// occluder that was already in front of the ray does not create a crossing.
fn trace(
    origin: vec3<f32>, ray: vec3<f32>, interval: vec2<f32>,
    body: OpticsBody, registered: bool,
) -> GatherHit {
    if interval.x >= interval.y { return empty_hit(); }
    let steps = select(32u, 16u, registered);
    let step = (interval.y - interval.x) / f32(steps);
    var previous = interval.x;
    var clear = false;
    for (var i = 0u; i <= steps; i++) {
        // Spend more of a scene trace near the water, where an edge moves
        // most in screen space. A constrained body interval is already short.
        let f = f32(i) / f32(steps);
        let t = mix(interval.x, interval.y, select(f * f, f, registered));
        let p = origin + ray * t;
        let projected = optics_project(p);
        if projected.z < 0.0 { return empty_hit(); }
        let hit = gather_at(projected.xy);
        if hit.distance == 0.0 {
            previous = t;
            clear = true;
            continue;
        }
        let delta = length(p - view.camera_xyz_time.xyz) - hit.distance;
        let eligible = select(hit.point.y > origin.y - 0.04, in_box(hit.point, body), registered);
        let projected_step = max(step, t - previous);
        if delta >= 0.0 && eligible && (clear || delta < 0.75 * projected_step + 0.04) {
            var left = previous;
            var right = t;
            var best = hit;
            for (var j = 0u; j < 5u; j++) {
                let middle = 0.5 * (left + right);
                let q = origin + ray * middle;
                let candidate = gather_at(optics_project(q).xy);
                if candidate.distance > 0.0 && length(q - view.camera_xyz_time.xyz) >= candidate.distance {
                    right = middle;
                    best = candidate;
                } else {
                    left = middle;
                }
            }
            let along = dot(best.point - origin, ray);
            let error = length(best.point - origin - along * ray);
            let tolerance = 0.045 + best.distance * 3.0 * max(
                view_aux.viewport_inv_viewport.z, view_aux.viewport_inv_viewport.w,
            );
            if along > 0.02 && along <= interval.y + 0.03 && error < tolerance
                && (!registered || in_box(best.point, body)) {
                return best;
            }
        }
        clear = delta < 0.0;
        previous = t;
    }
    return empty_hit();
}

fn trace_bodies(origin: vec3<f32>, ray: vec3<f32>, reach: f32, mask: u32) -> GatherHit {
    var remaining = mask & GATHER_BODY_MASK;
    var closest = reach;
    var best = empty_hit();
    while remaining != 0u {
        let index = firstTrailingBit(remaining);
        remaining &= remaining - 1u;
        let body = optics.bodies[index];
        let interval = box_interval(origin, ray, body, closest);
        let hit = trace(origin, ray, interval, body, true);
        if hit.distance > 0.0 {
            closest = dot(hit.point - origin, ray);
            best = hit;
        }
    }
    return best;
}

fn water_optical_paths(
    row: SurfaceRow, world: vec3<f32>, pixel: vec2<u32>, n: vec3<f32>, v: vec3<f32>,
    variance: f32, sky: vec3<f32>, fresnel: f32,
) -> OpticalPaths {
    let mask = optics_mask(pixel);
    let reach = optics.reach_slope_scene.x;
    let slope_limit = optics.reach_slope_scene.y;
    let slope_weight = smoothstep(slope_limit, mix(slope_limit, 1.0, 0.44), n.y);
    let reflected_ray = reflect(-v, n);
    let refracted_ray = refract(-v, n, 1.0 / 1.333);
    var reflected = empty_hit();
    var through = empty_hit();
    if (mask & GATHER_BODY_MASK) != 0u && slope_weight > 0.0 {
        reflected = trace_bodies(world, reflected_ray, reach, mask);
        through = trace_bodies(world, refracted_ray, optics.extinction_sight.w, mask);
    }
    var reflection_weight = slope_weight;
    let rough_limit = optics.downwelling_mirror.w;
    let still = 1.0 - smoothstep(rough_limit * 0.5, rough_limit, sqrt(variance));
    if reflected.distance == 0.0 && optics.reach_slope_scene.z != 0.0
        && still > 0.0 && reflected_ray.y > 0.0 {
        reflected = trace(world, reflected_ray, vec2(0.06, reach),
            OpticsBody(vec4(0.0), vec4(0.0)), false);
        reflection_weight = still;
    }
    var outgoing = sky;
    if reflected.distance > 0.0 {
        let t = length(reflected.point - world);
        let fade = reflected.edge * reflection_weight * (1.0 - smoothstep(0.75 * reach, reach, t));
        outgoing = mix(sky, fresnel * hit_light(reflected), fade);
    }

    let depth = max(0.0, world.y - row.bed);
    var thickness = depth / max(0.08, -refracted_ray.y);
    var behind = vec3(0.0);
    var moved = empty_hit();
    if through.distance == 0.0 && (mask & GATHER_SHALLOW) != 0u
        && depth < optics.extinction_sight.w {
        let straight = gather_at(optics_uv(pixel));
        if straight.distance > length(world - view.camera_xyz_time.xyz)
            && straight.point.y <= world.y + 0.005 {
            through = straight;
            let d = max(0.0, world.y - straight.point.y);
            // Distort the opaque lookup by the resolved slope and optical depth.
            // The unrefracted lookup remains the stable fallback at screen edges.
            let shift = n.xz * min(d, 4.0) * 0.35;
            let shifted = optics_project(straight.point + vec3(shift.x, 0.0, shift.y));
            let candidate = gather_at(shifted.xy);
            if candidate.distance > length(world - view.camera_xyz_time.xyz)
                && candidate.point.y <= world.y + 0.005 {
                moved = candidate;
            }
        }
    }
    if through.distance > 0.0 {
        thickness = length(through.point - world);
        let down = max(0.0, world.y - through.point.y);
        behind = hit_light(through) * exp(-optics.downwelling_mirror.xyz * down);
        if moved.distance > 0.0 && any(moved.pixel != through.pixel) {
            let separation = abs(moved.point.y - through.point.y);
            let weight = moved.edge * (1.0 - smoothstep(0.02, 0.18, separation));
            let moved_down = max(0.0, world.y - moved.point.y);
            let moved_light = hit_light(moved) * exp(-optics.downwelling_mirror.xyz * moved_down);
            behind = mix(behind, moved_light, weight);
            thickness = mix(thickness, length(moved.point - world), weight);
        }
    }
    return OpticalPaths(outgoing, behind, exp(-optics.extinction_sight.xyz * thickness), thickness);
}
