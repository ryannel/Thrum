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
    let warp = vec2(foam_noise(point * 3.7), foam_noise(point * 4.9 + 23.0)) - 0.5;
    let warped = point + 0.19 * warp;
    return mix(
        mix(pattern_patch(warped, tile, footprint), pattern_patch(warped, tile + vec2(1.0, 0.0), footprint), w.x),
        mix(pattern_patch(warped, tile + vec2(0.0, 1.0), footprint), pattern_patch(warped, tile + 1.0, footprint), w.x),
        w.y);
}
fn ranked_cover(stocks: vec4<f32>, point: vec2<f32>, footprint: f32) -> f32 {
    let cover = clamp(stocks.y + stocks.z, 0.0, 1.0);
    if cover < 0.0001 { return 0.0; }
    let rank = pattern_rank(point, footprint);
    let varied = mix(0.72, 1.20, foam_noise(point * vec2(0.69, 1.27)));
    let lace = smoothstep(rank - 0.10, rank + 0.10, varied * (0.70 * cover + 0.49 * stocks.y));
    let wind = foam_settings.time_wind.zw;
    let speed = length(wind);
    let along = select(vec2(1.0, 0.0), wind / max(0.001, speed), speed > 0.001);
    let streak_coords = vec2(dot(point, along), dot(point, vec2(-along.y, along.x)));
    let streak = smoothstep(0.22, 0.72, foam_noise(streak_coords * vec2(0.78, 3.60)));
    let broken = mix(0.16 + 0.84 * streak, 1.0, smoothstep(0.04, 0.22, stocks.y));
    let feather = sqrt(cover) * min(1.0, 1.55 * lace + 0.22 * cover) * broken;
    // A cap's body is churned water, not a flat sheet: its density varies.
    let churned = mix(0.72, 1.0, foam_noise(point * vec2(1.9, 2.3) + 11.0));
    let core = smoothstep(0.30, 0.90, stocks.y);
    let near = max(feather, core) * mix(1.0, churned, smoothstep(0.35, 0.85, cover));
    let filtered = max(cover, core) * mix(mix(1.0, churned, smoothstep(0.35, 0.85, cover)), 1.0,
        smoothstep(0.5, 4.0, footprint));
    return mix(near, filtered, smoothstep(0.16, 1.0, footprint)) * smoothstep(0.008, 0.028, cover);
}

// Renew a reading's pattern only at its own wrap, when its weight is
// zero. A steady obstruction then keeps shedding different foam shapes.
fn foam_renewal(cycle: u32, salt: f32) -> vec2<f32> {
    let seed = f32(cycle & 255u);
    return 2.0 * vec2(noise_hash(vec2(seed, salt)),
        noise_hash(vec2(seed, salt + 13.0))) - 1.0;
}

fn gaussian_lower_tail(x: f32) -> f32 {
    return 0.5 * (1.0 + sign(x) * sqrt(1.0 - exp(-0.63662 * x * x)));
}
fn foam_surface(point: vec3<f32>, row: SurfaceRow, footprint: f32, jacobian: f32, face: f32) -> vec4<f32> {
    let variance = max(0.001, dot(row.gains * row.gains, foam_settings.jacobian_variance));
    let wind = smoothstep(5.0, 18.0, length(foam_settings.time_wind.zw));
    let caps = wind * gaussian_lower_tail((0.82 - mix(jacobian, 1.0, smoothstep(1.0, 9.0, footprint))) / sqrt(variance));
    let shore = 1.0 - exp(-1.65 * row.loss);
    // Past the coarse level the caps continue with the mean the level held,
    // placed on crests by the estimator divided by its own expected mean.
    // The pixel's Jacobian and the unresolved spread each carry about the
    // spectrum's variance, so the estimator's mean is taken at twice it.
    let expected = wind * gaussian_lower_tail(-0.18 / sqrt(2.0 * variance));
    let held = foam_held();
    let placed = min(vec3(1.0), held.xyz * (caps / max(expected, 0.0001)));
    let sea = mix(vec3(0.04, 0.08, 0.10) * caps, placed, held.w);
    var far = vec4(sea.xy, sea.z + 0.34 * shore, 0.0);
    var distant_front = 0.0;
    if SURF_CACHE_ENABLED && row.loss > 0.0 {
        // Continue the distributed shore source beyond the moving windows.
        // At equilibrium the film spends 6 s fresh and 13 s aged.
        let bed = map_cell(point.xz);
        let birth = select(0.0, row.loss * 0.39, bed.inside && point.y > bed.bed);
        let air_birth = 0.22 * birth;
        let air = air_birth / (air_birth + 1.0 / 2.4);
        let film = clamp((birth + 0.60 * air / 2.4) / (birth + 1.0 / 19.0), 0.0, 1.0);
        // A mapped bed can end while swell is still losing power there.
        // Feather that estimate into open-sea cover before the bed ends.
        let grid = vec2<f32>(bed.corners[0]) + bed.uv;
        let edge = min(grid, vec2<f32>(map.size_count_sea.xy - 1u) - grid);
        let fade = smoothstep(vec2(0.0), vec2(12.0), edge);
        let shore_support = fade.x * fade.y;
        // Low diffuse cover leaves water visible between distant fronts.
        // Only the phase-resolved face draws the extra fresh film.
        let fresh = film * (6.0 / 19.0);
        let aged = film * (13.0 / 19.0);
        let diffuse_cover = 0.07;
        let shore_stocks = shore_support * vec3(0.35 * air, diffuse_cover * vec2(fresh, aged));
        distant_front = shore_support * (1.0 - sea.y) * fresh * (0.42 - diffuse_cover);
        far = vec4(sea + (vec3(1.0) - sea) * shore_stocks, 0.0);
    }
    let sample = foam_at(point.xz, far);
    var stocks = sample.stocks;
    let fresh_front = stocks.y + sample.distant_weight * distant_front;
    // Dense shallow-water foam still leaves gaps between groups of bubbles.
    // Weight the coarse breakup by depth-limited power so offshore caps
    // and the hull's own film keep their contact profile.
    let shore_mix = select(smoothstep(0.16, 0.95, row.loss),
        smoothstep(0.015, 0.12, row.loss), SURF_CACHE_ENABLED);
    if shore_mix > 0.0 {
        let holes = foam_noise(point.xz * vec2(0.65, 1.18));
        let porous = mix(0.23, 0.83, smoothstep(0.24, 0.78, holes));
        let shallow = mix(1.0, porous, shore_mix);
        // Aged wash opens first. Fresh film at the breaker still bleaches
        // the face before it spreads into the receding sheet.
        let cover = stocks.yz * select(vec2(shallow), vec2(mix(1.0, porous, 0.55 * shore_mix),
            shallow * mix(1.0, 0.55, shore_mix)), SURF_CACHE_ENABLED);
        stocks = vec4(stocks.x, cover, stocks.w);
    }
    // The stern ribbon carries submerged air alongside fresh film. In
    // the densest part, water shows between pockets of surface churn.
    let churn = max(smoothstep(0.010, 0.10, stocks.w),
        smoothstep(0.05, 0.36, stocks.x) * (1.0 - smoothstep(0.0, 5.0, length(foam_settings.time_wind.zw))));
    if churn > 0.0 {
        let bubbles = point.xz * vec2(1.33, 2.80) + vec2(1.8 * stocks.x, 2.3 * stocks.w);
        let pockets = 0.79 * foam_noise(bubbles) + 0.21 * foam_noise(bubbles * 2.63 + 17.0);
        let porous = mix(0.14, 0.94, smoothstep(0.32, 0.66, pockets));
        stocks = vec4(stocks.x, stocks.y * mix(1.0, porous, churn),
            stocks.z * mix(1.0, porous, churn), stocks.w);
    }
    // Submerged air gathers in clouds of bubbles. Its stock is smooth across a
    // trail, so tinting by it directly gives a pale band with ruled sides.
    // Modulating the stock by world-space clouds before the tint lets the
    // pale water's edge wander in and out across a metre or two.
    let clouds = 0.62 * foam_noise(point.xz * vec2(0.47, 0.58) + 31.0)
        + 0.38 * foam_noise(point.xz * vec2(1.31, 1.12) - 17.0);
    let air = stocks.x * smoothstep(0.06, 0.42, stocks.x * mix(0.30, 1.70, clouds));
