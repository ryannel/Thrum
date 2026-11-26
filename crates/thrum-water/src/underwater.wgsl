const WATER_INDEX = 1.333;

fn lens_point(uv: vec2<f32>) -> vec3<f32> {
    let p = optics_unproject(uv, 1.0);
    return p.xyz / p.w;
}

struct LensPoint {
    point: vec3<f32>,
    surface: f32,
    signed_depth: f32,
}

fn read_lens(uv: vec2<f32>) -> LensPoint {
    let point = lens_point(uv);
    let highest = highest_lens_face(point.xz);
    if highest <= reflection_bed(point.xz) {
        return LensPoint(point, point.y, -1.0);
    }
    return LensPoint(point, highest, highest - point.y);
}

// Evaluate the integral in a bounded form at either endpoint. Near c == K*m,
// use the series so the removable singularity does not lose precision.
fn water_path_integral(length_m: f32, start_depth: f32, end_depth: f32) -> vec3<f32> {
    let c = optics.extinction_sight.xyz;
    let k = optics.downwelling_mirror.xyz;
    let first = k * start_depth;
    let last = c * length_m + k * end_depth;
    let delta = abs(last - first);
    let small = 1.0 - 0.5 * delta + delta * delta / 6.0;
    let exact = (1.0 - exp(-delta)) / max(delta, vec3(0.00001));
    return length_m * exp(-min(first, last)) * select(exact, small, delta < vec3(0.01));
}

fn under_incident(point: vec3<f32>) -> vec3<f32> {
    return environment_diffuse(vec3(0.0, 1.0, 0.0))
        + water_sun(point) * max(0.0, environment_sun_direction().y);
}

fn water_path(
    light: vec3<f32>, incident: vec3<f32>, length_m: f32, start_depth: f32, end_depth: f32,
) -> vec3<f32> {
    let c = optics.extinction_sight.xyz;
    let scatter = incident * water.scatter.rgb * c / 3.14159265;
    return light * exp(-c * length_m)
        + scatter * water_path_integral(length_m, start_depth, end_depth);
}

// The derivative measures the height change per pixel across the line.
// Keep the dry side intact and grade the first covered pixel of the lens.
fn lens_coverage(depth: f32, pixel_height: f32) -> f32 {
    return smoothstep(0.0, max(pixel_height, 0.00001), depth);
}

fn lens_grazing_light(ray: vec3<f32>) -> vec3<f32> {
    return environment_sky_radiance(normalize(vec3(ray.x, 0.025, ray.z)));
}

@vertex fn underwater_vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(vec2(i & 1u, i >> 1u)) * 4.0 - 1.0;
    return vec4(p, 0.0, 1.0);
}

fn water_to_hit(eye: LensPoint, hit: GatherHit) -> vec3<f32> {
    let camera = view.camera_xyz_time.xyz;
    let reach = min(optics.extinction_sight.w * 4.0, 4000.0);
    let ray = normalize(eye.point - camera);
    let end = select(camera + ray * reach, hit.point, hit.distance > 0.0);
    let length_m = length(end - camera);
    let end_depth = max(0.0, eye.surface - end.y);
    let start_depth = max(0.0, eye.surface - camera.y);
    var body = vec3(0.0);
    if hit.distance > 0.0 {
        body = hit_light(hit) * exp(-optics.downwelling_mirror.xyz * end_depth);
    }
    let light = under_incident(vec3(eye.point.x, eye.surface, eye.point.z));
    return water_path(body, light, length_m, start_depth, end_depth);
}

fn water_to_gather(eye: LensPoint, uv: vec2<f32>) -> vec3<f32> {
    return water_to_hit(eye, gather_at(uv));
}

@fragment fn underwater_fragment(@builtin(position) clip: vec4<f32>) -> Output {
    let pixel = vec2<u32>(clip.xy);
    let uv = optics_uv(pixel);
    let eye = read_lens(uv);
    let coverage = lens_coverage(eye.signed_depth, fwidth(eye.signed_depth));
    if coverage == 0.0 { discard; }
    let medium = water_to_gather(eye, uv);
    let ray = normalize(eye.point - view.camera_xyz_time.xyz);
    let meniscus = 4.0 * coverage * (1.0 - coverage);
    let colour = mix(medium, lens_grazing_light(ray), meniscus * 0.65);
    return Output(vec4(colour * environment_storage_scale(), coverage), vec2(0.0), meniscus);
}

fn interface_fresnel(cosine: f32, transmitted: f32) -> f32 {
    let perpendicular = (WATER_INDEX * cosine - transmitted) / (WATER_INDEX * cosine + transmitted);
    let parallel = (cosine - WATER_INDEX * transmitted) / (cosine + WATER_INDEX * transmitted);
    return 0.5 * (perpendicular * perpendicular + parallel * parallel);
}

// The refracted ray can leave the camera image before reaching the mapped
// bank. Evaluate the terrain at its world intersection in that case.
fn transmitted_bank(point: vec3<f32>) -> vec3<f32> {
    let taps = vec2(0.5, 0.0);
    let dx = reflection_bed(point.xz + taps) - reflection_bed(point.xz - taps);
    let dz = reflection_bed(point.xz + taps.yx) - reflection_bed(point.xz - taps.yx);
    let normal = normalize(vec3(-dx, 1.0, -dz));
    let stony = smoothstep(0.07, 0.40, 1.0 - normal.y);
    let albedo = mix(vec3(0.19, 0.23, 0.105), vec3(0.37, 0.30, 0.19), stony);
    let sun = water_sun(point) * max(0.0, dot(environment_sun_direction(), normal));
    return albedo * (environment_diffuse(normal) + sun) / 3.14159265;
}

fn air_path_to(light: vec3<f32>, start: vec3<f32>, end: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let camera = view.camera_xyz_time.xyz;
    let end_air = environment_aerial(end, uv);
    let start_air = environment_aerial(camera + normalize(end - camera) * length(start - camera), uv);
    let inverse = 1.0 / max(start_air.transmission, vec3(0.001));
    return light * min(vec3(1.0), end_air.transmission * inverse)
        + max(vec3(0.0), end_air.scatter - start_air.scatter) * inverse;
}

fn underside_light(world: vec3<f32>, n: vec3<f32>, v: vec3<f32>, variance: f32, foam: f32, eye: LensPoint) -> vec3<f32> {
    let normal = select(n, -n, dot(n, v) < 0.0);
    let cosine = clamp(dot(normal, v), 0.0001, 1.0);
    let critical = sqrt(1.0 - 1.0 / (WATER_INDEX * WATER_INDEX));
    let rough = min(0.15, sqrt(variance) * 0.75);
    let boundary = max(0.001, rough);
    let window = smoothstep(critical - boundary, critical + boundary, cosine);
    let incident = under_incident(world);
    let reach = optics.reach_slope_scene.x;
    let scenery = OpticsBody(vec4(0.0), vec4(0.0));
    var transmitted = vec3(0.0);
    var transmission = 0.0;
    if window > 0.0 {
        let c = max(cosine, critical + 0.0001);
        let out_cos = sqrt(max(0.0, 1.0 - WATER_INDEX * WATER_INDEX * (1.0 - c * c)));
        let ray = normalize(-v * WATER_INDEX + (WATER_INDEX * cosine - out_cos) * normal);
        // The submerged reflection is limited by the water's sight distance.
        // An outgoing ray can still pick out terrain farther away in air.
        let air_reach = max(reach, 128.0);
        var bank = air_reach;
        if ray.y < 0.65 {
            bank = bank_crossing(world, ray, air_reach);
        }
        let hit = trace(world, ray, vec2(0.03, min(reach, bank + 0.1)), scenery, false, vec2(0.0), 1.0);
        let sky = environment_sky_radiance(normalize(vec3(ray.x, max(0.001, ray.y), ray.z)));
        transmitted = sky;
        if bank < air_reach {
            let point = world + ray * bank;
            transmitted = mix(sky, air_path_to(transmitted_bank(point), world, point, optics_project(world).xy),
                reach_weight(bank, air_reach));
        }
        if hit.distance > 0.0 {
            transmitted = mix(transmitted,
                air_path_to(hit_light(hit), world, hit.point, optics_uv(hit.pixel)), hit.edge);
        }
        transmission = window * (1.0 - interface_fresnel(c, out_cos));
    }
    let reflected = reflect(-v, normal);
    let reflection_reach = min(reach, optics.extinction_sight.w);
    let bank = bank_crossing(world, reflected, reflection_reach);
    let hit = trace(world, reflected, vec2(0.03, bank + 0.06), scenery, false, vec2(0.0), -1.0);
    let depth = max(0.0, -reflected.y * bank);
    var reflected_light = water_path(vec3(0.0), incident, bank, 0.0, depth);
    if hit.distance > 0.0 {
        let distance = length(hit.point - world);
        let end_depth = max(0.0, world.y - hit.point.y);
        let body = hit_light(hit) * exp(-optics.downwelling_mirror.xyz * end_depth);
        reflected_light = mix(reflected_light, water_path(body, incident, distance, 0.0, end_depth), hit.edge);
    }
    let light = mix(reflected_light, transmitted, transmission);
    let ceiling = incident * vec3(0.72, 0.83, 0.85) / 3.14159265;
    return water_path(mix(light, ceiling, foam), incident, length(world - view.camera_xyz_time.xyz),
        max(0.0, eye.surface - view.camera_xyz_time.y), 0.0);
}

fn water_underside(input: VertexOut, n: vec3<f32>, v: vec3<f32>, variance: f32, foam: f32) -> vec4<f32> {
    let uv = optics_uv(vec2<u32>(input.clip.xy));
    let eye = read_lens(uv);
    let pixel_depth = max(fwidth(eye.signed_depth), 0.00001);
    let clip_width = max(16.0 * fwidth(input.clip.z), 0.00001);
    let coverage = lens_coverage(eye.signed_depth, pixel_depth);
    if coverage == 0.0 { return vec4(0.0); }
    let hit = gather_at(uv);
    // A grazing sheet may briefly cross the line to a submerged opaque hit.
    // Keep the direct water path to that hit instead of replacing its
    // silhouette with the independently traced reflection or sky window.
    let depth_below_sheet = input.world.y - hit.point.y;
    let direct = select(0.0, smoothstep(0.0, 0.035, depth_below_sheet), hit.distance > 0.0);
    // Registered hulls span the air and water. At a grazing crossing the
    // opaque silhouette is a more stable endpoint than a reflected copy
    // of that same hull, which can jump between neighbouring wave facets.
    let hull = select(0.0, 1.0 - smoothstep(0.25, 0.5, -v.y),
        hit.distance > 0.0 && registered_source(hit));
    // At the near clip, the sheet gives way to the water path to the
    // opaque scene. Grade that handoff over the projected lens edge.
    let clip_weight = 1.0 - smoothstep(0.0, clip_width, 1.0 - input.clip.z);
    let line_weight = 1.0 - smoothstep(9.0, 16.0, eye.signed_depth / pixel_depth);
    let join = max(max(direct, hull), clip_weight * line_weight);
    var light = water_to_hit(eye, hit);
    if join < 1.0 {
        light = mix(underside_light(input.world, n, v, variance, foam, eye), light, join);
    }
    let colour = mix(lens_grazing_light(-v), light, coverage);
    return vec4(colour * environment_storage_scale(), coverage);
}
