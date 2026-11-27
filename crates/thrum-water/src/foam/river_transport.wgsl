
@compute @workgroup_size(8, 8)
fn prepare_transport(@builtin(global_invocation_id) id: vec3<u32>) {
    let level = id.z;
    let local = vec2<i32>(id.xy);
    let spacing = FOAM_SPACING[level];
    let old = foam_settings.origins[level].xy;
    let point = (vec2<f32>(old + local) + 0.5) * spacing;
    let dt = foam_settings.time_wind.x;
    let row = map_sample(point);
    var stocks = max(vec4(0.0), age_stocks(foam_load(foam_settings.banks.x, level, local), dt));
    var movement = vec4(0.0);
    if row.body != NO_BODY {
        let depth = select(1e30, row.row.water.x - row.bed, row.has_bed);
        let flow = map_flow(point, row.body);
        let swash = flow_swatch(row.row.gains * water.heights, flow.xy, map.swash.x, map.origin_spacing_margin.w);
        let gain = limit_wind_with_flow(row.row.gains, water.heights, depth, swash);
        let flow_gains = limit_flow(flow.xy, depth, swash);
        let dt_column = foam_settings.camera.z;
        let wake = wake_at(point, dt_column);
        var column: Wave;
        if row.has_bed {
            let current = row.row.water.yz;
            let track = map_wave_track(point, row.body, current);
            column = foam_column(point, gain, dt_column, current, track);
            column.displacement.y += foam_flow_height(point, current, track, flow_gains, spacing);
        }
        if foam_wet(level, local, depth + column.displacement.y + wake.x > 0.0) {
            let wave = foam_wave(point, gain, spacing);
            let energy_before = row.row.gains * water.heights;
            let loss = lost_wave_power(row.row.gains, gain, water.heights);
            let d = wave.differential;
            let compression = d.x * d.z - d.y * d.y;
            let wind = foam_settings.time_wind.zw;
            let crest = smoothstep(-0.08, 0.30, wave.height / max(water.scatter.w, 0.001));
            // Only the most compressed crests break, and they whiten fast, so a
            // gale holds bright caps on open water: about 15% steady cover in
            // the fine level, near the whitecap fractions measured at sea.
            let whitecap = pow(smoothstep(5.0, 18.0, length(wind)), 1.30) * crest *
                3.2 * smoothstep(0.54, 0.26, compression);
            let hull = hull_births(point, row.row.water.yz, spacing);
            let wake_break = 0.6 * max(0.0, length(wake.yz) - 0.46) *
                (1.0 - hull.z) * smoothstep(0.08, 0.30, abs(wake.w));
            let shore = shore_rates(level, local, loss * 0.39);
            let breaking = smoothstep(CHART_LOSS_BREAKING.x, CHART_LOSS_BREAKING.y, flow.z);
            let river_birth = min(3.3, flow.z * CHART_LOSS_BIRTH_RATE) * breaking * foam_settings.long_bounds.z;
            let rates = vec4(0.55 * whitecap + wake_break + shore.y,
                whitecap + wake_break + shore.x, 0.0, 0.0) + vec4(hull.xy, 0.0, hull.w) + river_birth * vec4(0.22, 1.0, 0.0, 0.14);
