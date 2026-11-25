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

fn gather_depth(pixel: vec2<u32>) -> f32 {
    if !optics_source(pixel) { return 0.0; }
    return textureLoad(gathered_distance, pixel, 0).x;
}

fn gather_point(pixel: vec2<u32>, distance: f32) -> vec3<f32> {
    let near_point = view_aux.inverse_view_proj * optics_ndc(optics_uv(pixel), 1.0);
    let camera = view.camera_xyz_time.xyz;
    let direction = normalize(near_point.xyz - camera * near_point.w);
    return camera + direction * distance;
}

fn gather_at(uv: vec2<f32>) -> GatherHit {
    if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) { return empty_hit(); }
    let pixel = vec2<u32>(uv * view_aux.viewport_inv_viewport.xy);
    let distance = gather_depth(pixel);
    if distance <= 0.0 { return empty_hit(); }
    let sides = min(uv, 1.0 - uv);
    let edge = smoothstep(0.0, 0.05, min(sides.x, sides.y));
    return GatherHit(gather_point(pixel, distance), distance, pixel, edge);
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

// A scene ray can see through the part of a registered body that fades.
fn registered_source(hit: GatherHit) -> bool {
    let count = u32(optics.reach_slope_scene.w);
    for (var index = 0u; index < count; index++) {
        let source = optics.sources[index];
        if all(hit.pixel >= source.xy) && all(hit.pixel < source.zw)
            && in_box(hit.point, optics.bodies[index]) {
            return true;
        }
    }
    return false;
}

fn trace_source(
    hit: GatherHit, origin: vec3<f32>, body: OpticsBody, registered: bool,
) -> bool {
    if registered { return in_box(hit.point, body); }
    return hit.point.y > origin.y - 0.04 && !registered_source(hit);
}

// The opaque depth at a hit must be near the ray itself. A foreground
// occluder that was already in front of the ray does not create a crossing.
fn trace(
    origin: vec3<f32>, ray: vec3<f32>, interval: vec2<f32>,
    body: OpticsBody, registered: bool,
) -> GatherHit {
    if interval.x >= interval.y { return empty_hit(); }
    // The scene's silhouettes need finer brackets over the full reach.
    // Registered geometry already restricts the trace to its short interval.
    let steps = select(48u, 16u, registered);
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
        if !registered && registered_source(hit) {
            previous = t;
            clear = true;
            continue;
        }
        let eligible = select(hit.point.y > origin.y - 0.04, in_box(hit.point, body), registered);
        let projected_step = max(step, t - previous);
        if delta >= 0.0 && eligible && (clear || delta < 0.75 * projected_step + 0.04) {
            var left = previous;
            var right = t;
            var best = hit;
            let refine = select(7u, 5u, registered);
            for (var j = 0u; j < refine; j++) {
                let middle = 0.5 * (left + right);
                let q = origin + ray * middle;
                let candidate = gather_at(optics_project(q).xy);
                if candidate.distance > 0.0 && length(q - view.camera_xyz_time.xyz) >= candidate.distance
                    && (registered || trace_source(candidate, origin, body, false)) {
                    right = middle;
                    best = candidate;
                } else {
                    left = middle;
                }
            }
            let along = dot(best.point - origin, ray);
            let error = length(best.point - origin - along * ray);
            let tolerance = select(0.018, 0.045, registered)
                + best.distance * select(1.8, 3.0, registered) * max(
                view_aux.viewport_inv_viewport.z, view_aux.viewport_inv_viewport.w,
            );
            if along > 0.02 && along <= interval.y + 0.03 && error < 2.0 * tolerance
                && (!registered || in_box(best.point, body)) {
                let agreement = 1.0 - smoothstep(0.75 * tolerance, 2.0 * tolerance, error);
                if registered {
                    best.edge *= agreement;
                } else {
                    best.edge = agreement;
                }
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

// Locate the first mapped bank crossing, including a crossing hidden from
// the camera. The depth march alone can step over it at a grazing angle.
fn bank_crossing(origin: vec3<f32>, ray: vec3<f32>, reach: f32) -> f32 {
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
        return hi;
    }
    return reach;
}

fn above_bank(point: vec3<f32>) -> f32 {
    return point.y - reflection_bed(point.xz);
}

// The scene gathers the full screen when it is admitted. All samples still
// go through gather_depth, including the silhouette probes.
fn bank_clearance(bank: vec3<f32>, projected: vec2<f32>) -> f32 {
    let size = view_aux.viewport_inv_viewport.xy;
    let centre = projected * size;
    let bank_distance = length(bank - view.camera_xyz_time.xyz);
    let radius = clamp(size.y / 240.0, 2.5, 6.0);
    let cell = vec2<i32>(floor(centre));
    let span = i32(ceil(radius));
    var obscurance = 0.0;
    for (var y = -span; y <= span; y++) {
        for (var x = -span; x <= span; x++) {
            let pixel = cell + vec2(x, y);
            if any(pixel < vec2(0)) || any(pixel >= vec2<i32>(size)) { continue; }
            let edge = max(abs(vec2<f32>(pixel) + 0.5 - centre) - 0.5, vec2(0.0));
            let support = 1.0 - smoothstep(0.0, radius, length(edge));
            if support <= obscurance { continue; }
            let distance = gather_depth(vec2<u32>(pixel));
            let nearer = smoothstep(0.35, 1.0, bank_distance - distance);
            if distance > 0.0 && nearer * support > obscurance {
                let height = above_bank(gather_point(vec2<u32>(pixel), distance));
                obscurance = max(obscurance, support * nearer * smoothstep(0.22, 0.5, height));
            }
        }
    }
    return 1.0 - obscurance;
}

// The foreground masks the grazing sky lobe as well as the bank. Broaden
// the fallback toward the open upper sky when low-angle sunlight lights the
// horizon, where the unoccluded directional lobe would give a bright stripe.
fn occluded_skylight(ray: vec3<f32>, sky: vec3<f32>, fresnel: f32) -> vec3<f32> {
    let up = vec3(0.0, 1.0, 0.0);
    let upper = fresnel * 0.7 * environment_sky_radiance(normalize(mix(ray, up, 0.88)));
    let low_sun = 1.0 - smoothstep(0.02, 0.35, environment_sun_direction().y);
    let light = mix(sky, upper, low_sun);
    let luminance = dot(light, vec3(0.2126, 0.7152, 0.0722));
    return mix(vec3(luminance), light, 0.25);
}

fn scene_edge(uv: vec2<f32>) -> f32 {
    let margin = min(uv, 1.0 - uv);
    return smoothstep(0.015, 0.14, min(margin.x, margin.y));
}

fn reach_weight(along: f32, reach: f32) -> f32 {
    return 1.0 - smoothstep(0.75 * reach, reach, along);
}

struct ReflectionLayer {
    hit: GatherHit,
    along: f32,
    weight: f32,
}

fn no_reflection(reach: f32) -> ReflectionLayer {
    return ReflectionLayer(empty_hit(), reach, 0.0);
}

fn reflect_layer(layer: ReflectionLayer, background: vec3<f32>, fresnel: f32) -> vec3<f32> {
    if layer.weight <= 0.0 { return background; }
    return mix(background, fresnel * hit_light(layer.hit), layer.weight);
}

// Very thin foreground depth at a disocclusion can otherwise leave an
// opaque streak in the fallback. Estimate vertical silhouette coverage
// from depth on either side of the hit without fetching either colour.
fn foreground_coverage(hit: GatherHit) -> f32 {
    let size = optics_extent();
    let spacing = min(max(size.y / 720u, 1u), hit.pixel.y);
    let below = min(hit.pixel.y + spacing, size.y - 1u);
    let tolerance = 0.07 + 4.0 * hit.distance * view_aux.viewport_inv_viewport.w;
    let above_depth = gather_depth(vec2(hit.pixel.x, hit.pixel.y - spacing));
    let below_depth = gather_depth(vec2(hit.pixel.x, below));
    let support = vec2(above_depth, below_depth);
    let same_surface = (1.0 - smoothstep(vec2(tolerance), vec2(2.0 * tolerance),
        abs(support - hit.distance))) * step(vec2(0.00001), support);
    return mix(0.25, 1.0, max(same_surface.x, same_surface.y));
}

struct SceneReflection {
    layer: ReflectionLayer,
    environment: vec3<f32>,
}

fn mapped_agreement(hit: GatherHit, point: vec3<f32>) -> f32 {
    if hit.distance <= 0.0 || registered_source(hit) { return 0.0; }
    let height = abs(above_bank(hit.point));
    let offset = length(hit.point.xz - point.xz);
    return (1.0 - smoothstep(0.21, 0.32, height)) * (1.0 - smoothstep(1.4, 2.0, offset));
}

fn scene_reflection(
    origin: vec3<f32>, ray: vec3<f32>, reach: f32, sky: vec3<f32>, fresnel: f32, still: f32,
    body_layer: ReflectionLayer,
) -> SceneReflection {
    let limit = select(reach, body_layer.along, body_layer.weight >= 1.0);
    if limit <= 0.06 { return SceneReflection(no_reflection(reach), sky); }
    let crossing = bank_crossing(origin, ray, limit);
    let mapped = crossing < limit;
    let point = origin + ray * crossing;
    let projected = optics_project(point);
    let bank_border = scene_edge(projected.xy);
    var bank = empty_hit();
    var agreement = 0.0;
    var clearance = 1.0;
    if mapped && projected.z >= 0.0 && bank_border > 0.0 {
        bank = gather_at(projected.xy);
        agreement = mapped_agreement(bank, point);
        clearance = bank_clearance(point, projected.xy);
    }

    let scenery = OpticsBody(vec4(0.0), vec4(0.0));
    var hit = trace(origin, ray, vec2(0.06, limit), scenery, false);
    var along = select(crossing, dot(hit.point - origin, ray), hit.distance > 0.0);
    var border = scene_edge(optics_uv(hit.pixel));
    var strength = hit.edge;
    var foreground_shadow = 0.0;
    if hit.distance > 0.0 && mapped {
        let height = above_bank(hit.point);
        let ground = (1.0 - smoothstep(0.20, 0.36, abs(height)))
            * (1.0 - smoothstep(1.0, 2.5, abs(along - crossing)));
        // Near the silhouette the march can find a point just off the ray.
        // Give that point zero coverage before the mapped ground takes over.
        let uncertain = agreement * (1.0 - ground);
        strength *= mix(1.0, smoothstep(0.21, 0.55, hit.edge), uncertain);
        // A partially covered foreground can obscure the sky even when
        // its projected outline misses the mapped bank lookup.
        foreground_shadow = smoothstep(0.02, 0.28, hit.edge) * smoothstep(0.16, 0.45, height);
        let before = 1.0 - smoothstep(0.0, 0.4, along - crossing);
        strength *= mix(max(before, ground * clearance), 1.0, clearance);
        if agreement > 0.0 && ground > 0.0 {
            // Close estimates of the same ground refine one lookup.
            // Interpolate the source position and evaluate its colour once.
            let contribution = agreement * ground * (1.0 - strength);
            let uv = mix(optics_uv(hit.pixel), projected.xy, contribution);
            let resolved = gather_at(uv);
            strength = max(strength, agreement * ground);
            if resolved.distance <= 0.0 || any(resolved.pixel != hit.pixel) {
                // Changing the texel must not pick up a foreground colour
                // between the two estimates of the bank.
                strength *= mapped_agreement(resolved, point);
            }
            hit = resolved;
            along = mix(along, crossing, contribution);
            border = scene_edge(uv);
        }
        strength *= mix(mix(foreground_coverage(hit), 1.0, clearance), clearance, ground);
    }
    if strength <= 0.0 && agreement > 0.0 {
        hit = bank;
        along = crossing;
        strength = agreement * clearance;
        border = bank_border;
        foreground_shadow = 0.0;
    }

    let weight = select(0.0, still * reach_weight(along, reach) * border * strength,
        hit.distance > 0.0);
    var environment = sky;
    let shadow = max(foreground_shadow, smoothstep(0.0, 0.55, 1.0 - clearance));
    if shadow > 0.0 {
        let shade = still * reach_weight(crossing, reach) * bank_border * shadow;
        // Reserve the obscured part of the lobe for the muted sky even
        // when most of the same ground is still visible.
        let below = min(1.0, shade / max(1.0 - weight, 0.00001));
        environment = mix(sky, occluded_skylight(ray, sky, fresnel), below);
    }
    return SceneReflection(ReflectionLayer(hit, along, weight), environment);
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
    let rough_limit = optics.downwelling_mirror.w;
    let still = 1.0 - smoothstep(rough_limit * 0.5, rough_limit, sqrt(variance));
    var body = no_reflection(reach);
    if reflected.distance > 0.0 {
        let along = dot(reflected.point - world, reflected_ray);
        let weight = reflected.edge * slope_weight * reach_weight(along, reach);
        body = ReflectionLayer(reflected, along, weight);
    }
    var scene = SceneReflection(no_reflection(reach), sky);
    if optics.reach_slope_scene.z != 0.0 && still > 0.0 && reflected_ray.y > 0.0 {
        scene = scene_reflection(world, reflected_ray, reach, sky, fresnel, still, body);
    }
    var outgoing: vec3<f32>;
    if body.along < scene.layer.along {
        outgoing = reflect_layer(body, reflect_layer(scene.layer, scene.environment, fresnel), fresnel);
    } else {
        outgoing = reflect_layer(scene.layer, reflect_layer(body, scene.environment, fresnel), fresnel);
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
