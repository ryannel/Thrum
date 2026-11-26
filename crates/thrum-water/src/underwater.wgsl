struct LensColumns {
    state: vec4<u32>,
    columns: array<array<vec4<f32>, 33>, 33>,
}
@group(3) @binding(15) var<uniform> lens: LensColumns;

const WATER_INDEX = 1.333;

fn lens_point(uv: vec2<f32>) -> vec3<f32> {
    let p = view_aux.inverse_view_proj * optics_ndc(uv, 1.0);
    return p.xyz / p.w;
}

fn lens_column(uv: vec2<f32>) -> vec4<f32> {
    let last = lens.state.yz - vec2(1u);
    let coord = clamp(uv, vec2(0.0), vec2(1.0)) * vec2<f32>(last);
    let cell = vec2<u32>(coord);
    let next = min(cell + vec2(1u), last);
    let f = fract(coord);
    let a = lens.columns[cell.y][cell.x];
    let b = lens.columns[cell.y][next.x];
    let c = lens.columns[next.y][cell.x];
    let d = lens.columns[next.y][next.x];
    if min(min(a.w, b.w), min(c.w, d.w)) == 0.0 {
        let nearest = vec2<u32>(round(coord));
        return lens.columns[nearest.y][nearest.x];
    }
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

struct LensPoint {
    point: vec3<f32>,
    surface: f32,
    signed_depth: f32,
}

fn read_lens(uv: vec2<f32>) -> LensPoint {
    let point = lens_point(uv);
    let column = lens_column(uv);
    if column.w == 0.0 {
        return LensPoint(point, point.y, -1.0);
    }
    let row = lens_row(point.xz);
    let motion = cascade_motion(point.xz, row.current, row.track, water.carriage.w, water.carriage.y);
    var height = column.x + wake_at(point.xz, 0.0).x;
    for (var band = 0u; band < 3u; band++) {
        if row.gains[band + 1u] * water.heights[band + 1u] > 0.0 {
            height += row.gains[band + 1u] * transported_field(motion, band).displacement.y;
        }
    }
    let depth = height - point.y;
    return LensPoint(point, height, select(-abs(depth), depth, height > column.y && row.owned));
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

@fragment fn underwater_fragment(@builtin(position) clip: vec4<f32>) -> Output {
    let pixel = vec2<u32>(clip.xy);
    let uv = optics_uv(pixel);
    let eye = read_lens(uv);
    let coverage = lens_coverage(eye.signed_depth, fwidth(eye.signed_depth));
    if coverage == 0.0 { discard; }
    let hit = gather_at(uv);
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
    let medium = water_path(body, light, length_m, start_depth, end_depth);
    let meniscus = 4.0 * coverage * (1.0 - coverage);
    let colour = mix(medium, lens_grazing_light(ray), meniscus * 0.65);
    return Output(vec4(colour * environment_storage_scale(), coverage), vec2(0.0), meniscus);
}

fn interface_fresnel(cosine: f32, transmitted: f32) -> f32 {
    let perpendicular = (WATER_INDEX * cosine - transmitted) / (WATER_INDEX * cosine + transmitted);
    let parallel = (cosine - WATER_INDEX * transmitted) / (cosine + WATER_INDEX * transmitted);
    return 0.5 * (perpendicular * perpendicular + parallel * parallel);
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
        let hit = trace(world, ray, vec2(0.03, reach), scenery, false, vec2(0.0), 1.0);
        let sky = environment_sky_radiance(normalize(vec3(ray.x, max(0.001, ray.y), ray.z)));
        transmitted = sky;
        if hit.distance > 0.0 {
            let source_uv = optics_uv(hit.pixel);
            let end_air = environment_aerial(hit.point, source_uv);
            let start_air = environment_aerial(view.camera_xyz_time.xyz
                + normalize(hit.point - view.camera_xyz_time.xyz) * length(world - view.camera_xyz_time.xyz), source_uv);
            let inverse = 1.0 / max(start_air.transmission, vec3(0.001));
            let air = EnvironmentAerial(min(vec3(1.0), end_air.transmission * inverse),
                max(vec3(0.0), end_air.scatter - start_air.scatter) * inverse);
            transmitted = mix(sky, hit_light(hit) * air.transmission + air.scatter, hit.edge);
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
    if lens.state.x == 0u { return vec4(0.0); }
    let uv = optics_uv(vec2<u32>(input.clip.xy));
    let eye = read_lens(uv);
    let coverage = lens_coverage(eye.signed_depth, fwidth(eye.signed_depth));
    if eye.signed_depth <= 0.0 { return vec4(0.0); }
    let light = underside_light(input.world, n, v, variance, foam, eye);
    let colour = mix(lens_grazing_light(-v), light, coverage);
    return vec4(colour * environment_storage_scale(), 1.0);
}
