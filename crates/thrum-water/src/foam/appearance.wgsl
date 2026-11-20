const FOAM_ENABLED: bool = true;
@group(3) @binding(12) var foam_pattern: texture_2d<f32>;

fn noise_hash(at: vec2<f32>) -> f32 {
    var p = fract(vec3(at.x, at.y, at.x) * vec3(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}
fn foam_noise(p: vec2<f32>) -> f32 {
    let tile = floor(p);
    let f = fract(p);
    let w = f*f*(3.0 - 2.0*f);
    return mix(mix(noise_hash(tile), noise_hash(tile + vec2(1.0, 0.0)), w.x),
        mix(noise_hash(tile + vec2(0.0, 1.0)), noise_hash(tile + 1.0), w.x), w.y);
}
fn pattern_patch(point: vec2<f32>, tile: vec2<f32>, footprint: f32) -> f32 {
    let angle = 6.2831853 * noise_hash(tile + 43.0);
    let sc = vec2(cos(angle), sin(angle));
    let local = point - tile * 3.2;
    let uv = vec2(dot(local, sc), dot(local, vec2(-sc.y, sc.x))) * 1.08
        + vec2(noise_hash(tile + 7.0), noise_hash(tile - 29.0));
    let mip = max(0.0, log2(max(1.0, 1.08 * footprint * f32(textureDimensions(foam_pattern).x))));
    return textureSampleLevel(foam_pattern, periodic, uv, mip).a;
}
fn pattern_rank(point: vec2<f32>, footprint: f32) -> f32 {
    if textureDimensions(foam_pattern).x <= 1u {
        return 0.68 * foam_noise(point * 2.4) + 0.32 * foam_noise(point * 8.7);
    }
    let tile = floor(point / 3.2);
    let f = fract(point / 3.2);
    let w = smoothstep(vec2(0.30), vec2(0.70), f);
    return mix(
        mix(pattern_patch(point, tile, footprint), pattern_patch(point, tile + vec2(1.0, 0.0), footprint), w.x),
        mix(pattern_patch(point, tile + vec2(0.0, 1.0), footprint), pattern_patch(point, tile + 1.0, footprint), w.x),
        w.y);
}
fn ranked_cover(stocks: vec4<f32>, point: vec2<f32>, footprint: f32) -> f32 {
    let cover = clamp(stocks.y + stocks.z, 0.0, 1.0);
    if cover < 0.0001 { return 0.0; }
    let rank = pattern_rank(point, footprint);
    let varied = mix(0.72, 1.20, foam_noise(point * vec2(0.69, 1.27)));
    let lace = smoothstep(rank - 0.10, rank + 0.10, varied * (0.70 * cover + 0.49 * stocks.y));
    let feather = sqrt(cover) * min(1.0, 1.8 * lace + 0.30 * cover);
    let core = smoothstep(0.19, 0.59, stocks.y);
    let near = max(feather, core);
    return mix(near, max(cover, core), smoothstep(0.16, 1.0, footprint)) * smoothstep(0.008, 0.028, cover);
}

fn gaussian_lower_tail(x: f32) -> f32 {
    return 0.5 * (1.0 + sign(x) * sqrt(1.0 - exp(-0.63662 * x * x)));
}
fn foam_surface(point: vec3<f32>, row: SurfaceRow, footprint: f32, jacobian: f32) -> vec4<f32> {
    let variance = max(0.001, dot(row.gains * row.gains, foam_settings.jacobian_variance));
    let wind = smoothstep(5.0, 18.0, length(foam_settings.time_wind.zw));
    let caps = wind * gaussian_lower_tail((0.82 - mix(jacobian, 1.0, smoothstep(1.0, 9.0, footprint))) / sqrt(variance));
    let shore = 1.0 - exp(-6.72 * row.loss);
    let far = vec4(0.04 * caps, 0.08 * caps, 0.10 * caps + 0.34 * shore, 0.0);
    let stocks = foam_at(point.xz, far);
    return vec4(stocks.x, ranked_cover(stocks, point.xz, footprint), stocks.zw);
}

@group(1) @binding(0) var shore_depth: texture_depth_2d;

@vertex fn film_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment fn film_fragment(@builtin(position) pixel: vec4<f32>) -> Output {
    let uv = pixel.xy * view_aux.viewport_inv_viewport.zw;
    let depth = textureLoad(shore_depth, vec2<i32>(pixel.xy), 0);
    if depth < 1e-7 { discard; }
    let clip = vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), depth, 1.0);
    let homogeneous = view_aux.inverse_view_proj * clip;
    let ground = homogeneous.xyz / max(homogeneous.w, 1e-9);
    let footprint = length(ground - view.camera_xyz_time.xyz) * view_aux.viewport_inv_viewport.w;
    let answer = map_sample(ground.xz);
    if answer.body == NO_BODY || !answer.has_bed { discard; }
    let row = surface_row(map_cell(ground.xz), answer.row, answer.body, true);
    let reach = 0.175 * row.swash;
    let above = row.bed - row.level;
    if reach <= 0.0 || above >= reach || above < -0.27 * row.swash { discard; }
    // The depth already on screen must belong to the ground. A hull, a hill
    // in front of the beach, or the visible sheet above the bed masks the film.
    if abs(ground.y - row.bed) > 0.06 { discard; }
    let field = field_at(ground.xz, 0.0, row.gains);
    let thickness = row.level + field.displacement.y + wake_at(ground.xz, 0.0).x - row.bed;
    if thickness > 0.0 { discard; }
    let sheen = (1.0 - smoothstep(0.0, reach, above)) * smoothstep(0.0, 0.04, -thickness);
    let stocks = foam_at(ground.xz, vec4(0.0));
    let foam = ranked_cover(stocks, ground.xz, footprint);
    let v = normalize(view.camera_xyz_time.xyz - ground);
    let ray = reflect(-v, vec3(0.0, 1.0, 0.0));
    let sky = environment_sky_radiance(ray);
    let fresnel = water_fresnel(v.y);
    let incident = environment_diffuse(vec3(0.0, 1.0, 0.0)) + sun_irradiance(ground) * max(0.0, environment_sun_direction().y);
    let light = mix(sky, 0.24 * incident, foam);
    let air = environment_aerial(ground, uv);
    let colour = (light * air.transmission + air.scatter) * environment_storage_scale();
    return Output(vec4(colour, sheen * clamp(fresnel * 0.34 + foam, 0.0, 1.0)), vec2(0.0), 0.0);
}
