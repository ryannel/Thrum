const OPTICS_ENABLED: bool = true;
@group(1) @binding(3) var gathered_radiance: texture_2d<f32>;
@group(1) @binding(4) var gathered_distance: texture_2d<f32>;

struct OpticalPaths {
    reflected: vec3<f32>,
    behind: vec3<f32>,
    transmission: vec3<f32>,
    edge_depth: f32,
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
    if !optics_source(pixel) { return empty_hit(); }
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
            if along > 0.02 && along <= interval.y + 0.03 && error < 2.0 * tolerance
                && (!registered || in_box(best.point, body)) {
                best.edge *= 1.0 - smoothstep(0.75 * tolerance, 2.0 * tolerance, error);
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

// Continue a narrow gap in visible bank radiance across a foreground
// silhouette. The bed locates the ray's crossing, and only opaque samples
// whose depth agrees with nearby bed can contribute its colour.
fn fill_bank_gap(origin: vec3<f32>, ray: vec3<f32>, reach: f32) -> GatherHit {
    var previous = 0.06;
    for (var i = 1u; i <= 24u; i++) {
        let phase = f32(i) / 24.0;
        let t = mix(0.06, reach, phase * phase);
        let p = origin + ray * t;
        if p.y > reflection_bed(p.xz) { previous = t; continue; }

        var lo = previous;
        var hi = t;
        for (var j = 0u; j < 7u; j++) {
            let mid = (lo + hi) * 0.5;
            let q = origin + ray * mid;
            if q.y > reflection_bed(q.xz) { lo = mid; } else { hi = mid; }
        }
        let bank = origin + ray * hi;
        let projected = optics_project(bank);
        let margin = min(projected.xy, 1.0 - projected.xy);
        if projected.z < 0.0 || min(margin.x, margin.y) < 0.015 { break; }
        let forward = normalize(ray.xz);
        let across = vec2(-forward.y, forward.x);
        let directions = array(across, -across, forward, -forward);
        for (var j = 0u; j <= 16u; j++) {
            let radius = f32((j + 3u) / 4u);
            let offset = directions[(j + 3u) % 4u] * radius;
            let xz = bank.xz + offset;
            let bed = vec3(xz.x, reflection_bed(xz), xz.y);
            let location = optics_project(bed);
            if location.z < 0.0 { continue; }
            var sample = gather_at(location.xy);
            // At a grazing view, one texel can span a long strip of ground.
            // Test its reconstructed point against the bed at that point.
            if sample.distance > 0.0 && length(sample.point.xz - xz) < 1.8
                && abs(sample.point.y - reflection_bed(sample.point.xz)) < 0.22 {
                sample.point = bank;
                return sample;
            }
        }
        break;
    }
    return empty_hit();
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
    let depth = max(0.0, world.y - row.bed);
    var bank = empty_hit();
    var reflection_weight = slope_weight;
    var scene_hit = false;
    let rough_limit = optics.downwelling_mirror.w;
    let still = 1.0 - smoothstep(rough_limit * 0.5, rough_limit, sqrt(variance));
    if reflected.distance == 0.0 && optics.reach_slope_scene.z != 0.0
        && still > 0.0 && reflected_ray.y > 0.0 {
        let scenery = OpticsBody(vec4(0.0), vec4(0.0));
        let segment = vec2(0.06, reach);
        reflected = trace(world, reflected_ray, segment, scenery, false);
        reflection_weight = still;
        if reflected.edge < 0.995 {
            bank = fill_bank_gap(world, reflected_ray, reach);
        }
        scene_hit = true;
    }
    var outgoing = sky;
    if bank.distance > 0.0 {
        let dist = length(bank.point - world);
        let uv = optics_project(bank.point).xy;
        let margin = min(uv, 1.0 - uv);
        let edge = smoothstep(0.015, 0.14, min(margin.x, margin.y));
        let fade = edge * reflection_weight * (1.0 - smoothstep(0.75 * reach, reach, dist));
        outgoing = mix(sky, fresnel * hit_light(bank), fade);
    }
    if reflected.distance > 0.0 {
        let t = length(reflected.point - world);
        let uv = optics_uv(reflected.pixel);
        let margin = min(uv, 1.0 - uv);
        let scene_edge = min(reflected.edge, smoothstep(0.015, 0.14, min(margin.x, margin.y)));
        let fade = select(reflected.edge, scene_edge, scene_hit) * reflection_weight
            * (1.0 - smoothstep(0.75 * reach, reach, t));
        outgoing = mix(outgoing, fresnel * hit_light(reflected), fade);
    }

    var edge_depth = depth;
    let surface_distance = length(world - view.camera_xyz_time.xyz);
    var straight = empty_hit();
    if (mask & (GATHER_SHALLOW | GATHER_BODY_MASK)) != 0u {
        let sample = gather_at(optics_uv(pixel));
        if sample.distance > surface_distance && sample.point.y <= world.y + 0.005 {
            straight = sample;
            edge_depth = min(edge_depth, max(0.0, world.y - sample.point.y));
        }
    }
    var thickness = depth / max(0.08, -refracted_ray.y);
    var behind = vec3(0.0);
    var moved = empty_hit();
    if through.distance == 0.0 && (mask & GATHER_SHALLOW) != 0u
        && depth < optics.extinction_sight.w {
        if straight.distance > 0.0 {
            through = straight;
            let d = max(0.0, world.y - straight.point.y);
            // Distort the opaque lookup by the resolved slope and optical depth.
            // The unrefracted lookup remains the stable fallback at screen edges.
            let shift = n.xz * min(d, 4.0) * 0.35;
            let shifted = optics_project(straight.point + vec3(shift.x, 0.0, shift.y));
            let candidate = gather_at(shifted.xy);
            if candidate.distance > surface_distance
                && candidate.point.y <= world.y + 0.005 {
                moved = candidate;
            }
        }
    }
    if through.distance > 0.0 {
        thickness = length(through.point - world);
        let down = max(0.0, world.y - through.point.y);
        edge_depth = min(edge_depth, down);
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
    return OpticalPaths(outgoing, behind, exp(-optics.extinction_sight.xyz * thickness), edge_depth);
}
