struct Parcel {
    stocks: vec4<f32>,
    movement: vec4<f32>, // displacement in cells, diffusivity weight, shore birth rate
}
@group(0) @binding(11) var<storage, read_write> parcels: array<Parcel>;

struct FilteredWave {
    height: f32,
    differential: vec3<f32>,
}
fn foam_wave(point: vec2<f32>, gains: vec4<f32>, spacing: f32) -> FilteredWave {
    var wave = FilteredWave(0.0, vec3(1.0, 0.0, 1.0));
    for (var band = 0u; band < 4u; band++) {
        let gain = gains[band];
        if gain * water.heights[band] <= 0.0 { continue; }
        var surface: vec4<f32>;
        var derivative: vec4<f32>;
        var rate: vec4<f32>;
        if band == 0u {
            let radius = length(point - foam_settings.camera.xy);
            let level = u32(clamp(ceil(log2(max(1.0, radius / 62.0))), 0.0, 7.0));
            let uv = (point - water.cache[level].xy) / (128.0 * water.cache[level].z);
            let layer = i32(5u * level);
            surface = textureSampleLevel(long_field, bounded, uv, layer, 0.0);
            derivative = textureSampleLevel(long_field, bounded, uv, layer + 1, 0.0);
            rate = textureSampleLevel(long_field, bounded, uv, layer + 2, 0.0);
        } else {
            let length_m = water.lengths[band - 1u];
            let uv = point / length_m;
            let layer = i32(5u * (band - 1u));
            let lod = max(0.0, log2(128.0 * spacing / length_m));
            surface = textureSampleLevel(cascades, periodic, uv, layer, lod);
            derivative = textureSampleLevel(cascades, periodic, uv, layer + 1, lod);
            rate = textureSampleLevel(cascades, periodic, uv, layer + 2, lod);
        }
        wave.height += gain * surface.y;
        wave.differential += gain * vec3(derivative.zw, rate.w);
    }
    return wave;
}

fn air_fraction(dt: f32) -> f32 { return exp(-dt / 2.4); }
fn age_stocks(stocks: vec4<f32>, dt: f32) -> vec4<f32> {
    let air = air_fraction(dt);
    let fresh = exp(-dt / 6.0);
    let aged = exp(-dt / 13.0);
    let ab = (fresh - air) * 6.0 / (6.0 - 2.4);
    let bc = (aged - fresh) * 13.0 / (13.0 - 6.0);
    // Integrate the fresh-to-aged flux of air that surfaced during this step.
    let ac = (bc - (2.4 / 6.0) * 13.0 / (13.0 - 2.4) * (aged - air)) * 6.0 / (6.0 - 2.4);
    return vec4(stocks.x * air, stocks.y * fresh + 0.60 * stocks.x * ab,
        stocks.z * aged + stocks.y * bc + 0.60 * stocks.x * ac, stocks.w * exp(-dt / 8.0));
}
fn add_births(stocks: vec4<f32>, rates: vec4<f32>, dt: f32) -> vec4<f32> {
    let room = max(0.0, 1.0 - stocks.y - stocks.z);
    return stocks + max(vec4(0.0), vec4(1.0 - stocks.x, room, 0.0, 1.0 - stocks.w)) * (1.0 - exp(-rates * dt));
}
fn hull_births(point: vec2<f32>, current: vec2<f32>, spacing: f32) -> vec4<f32> {
    var rates = vec4(0.0);
    for (var index = 0u; index < foam_settings.banks.z; index++) {
        let hull = foam_settings.hulls[index];
        let at = point - hull.location_axis.xy + 0.5 * foam_settings.time_wind.x * hull.velocity.xy;
        let ext = hull.extent_rate.xy;
        if dot(at, at) > pow(5.0 + ext.x, 2.0) { continue; }
        // A fixed obstruction sheds on the downstream side of the current.
        // Moving hulls retain their contact and stern sources.
        let flow = length(current);
        if all(hull.velocity.xy == vec2(0.0)) && flow > 0.0 {
            let downstream = current / flow;
            let along = dot(at, downstream);
            let across = dot(at, vec2(-downstream.y, downstream.x));
            let behind = along - ext.x * 0.5;
            if behind > 0.0 {
                let core = exp(-0.85 * behind - pow(across / (ext.y + 0.14 * behind), 2.0));
                let birth = min(3.3, 1.45 * flow * flow) * core;
                rates += vec4(0.60, 0.72, 0.0, 0.14) * birth;
            }
            continue;
        }
        let direction = hull.location_axis.zw;
        let local = vec2(dot(at, direction), dot(at, vec2(-direction.y, direction.x)));
        let speed = max(0.0, length(hull.velocity.xy - current) - 0.22);
        let edge = length(local / (ext + spacing * 0.25));
        let contact = exp(-16.0 * (edge - 0.85) * (edge - 0.85)) * speed * speed;
        let aft = max(0.0, -local.x - ext.x * 0.7);
        let stern = select(0.0, exp(-0.85 * aft - pow(local.y / (ext.y + 0.35 + aft * 0.13), 2.0)), aft >= 0.0);
        let work = select(0.0, max(0.0, hull.extent_rate.z - 25.0) * 0.00016 * stern, local.x <= -ext.x * 0.7);
        rates += vec4(0.6 * contact + work, 0.32 * contact + 0.8 * work, 0.0, 0.14 * work);
        // The stationary pressure dent and its settling transient carry no
        // horizontal drag power. Exclude that footprint from wake breaking.
        if speed == 0.0 && work == 0.0 {
            rates.z = max(rates.z, 1.0 - smoothstep(1.55, 2.45, edge));
        }
    }
    return rates;
}

fn swash_bed_slope(cell: Cell) -> vec2<f32> {
    let h = cell.heights;
    return vec2(mix(h.y - h.x, h.w - h.z, cell.uv.y),
        mix(h.z - h.x, h.w - h.y, cell.uv.x)) / map.origin_spacing_margin.z;
}

fn swash_drift(slope: vec2<f32>, upward: f32, depth: f32, swash: f32) -> vec2<f32> {
    let speed = min(sqrt(9.81 * swash), max(0.0, upward) / max(0.001, length(slope)));
    return -normalize(slope + vec2(1e-8)) * speed * (1.0 - smoothstep(0.0, 0.27 * swash, depth));
}

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
        let gain = limit_gains(row.row.gains, depth, water.heights, map.swash.x, map.origin_spacing_margin.w);
        let dt_column = foam_settings.camera.z;
        let wake = wake_at(point, dt_column);
        var column: Wave;
        if row.has_bed {
            column = foam_column(point, gain, dt_column, row.row.water.yz);
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
            let rates = vec4(0.55 * whitecap + wake_break + shore.y,
                whitecap + wake_break + shore.x, 0.0, 0.0) + vec4(hull.xy, 0.0, hull.w);
            stocks = add_births(stocks, rates, dt);
            var drift = row.row.water.yz + 0.027 * wind;
            let swash = min(map.swash.x * length(energy_before), map.origin_spacing_margin.w);
            if level == 0u && row.has_bed && swash > 0.0 && depth < 0.27 * swash {
                let thickness_slope = column.slope + wake.yz - swash_bed_slope(map_cell(point));
                let plain = swash_drift(thickness_slope, column.velocity.y + wake.w, depth, swash);
                drift += shore_uprush(level, local, plain);
            }
            let max_shift = select(0.999, 2.0, level == 0u && dt > 0.024);
            let shift = clamp(drift * dt / spacing, vec2(-max_shift), vec2(max_shift));
            let diffusion = min(0.24, (0.016 + 0.14 * stocks.x + 0.30 * stocks.y) * dt / (spacing * spacing));
            movement = vec4(shift, diffusion, shore.x);
        } else if level == 0u && stranded_shore(local) {
            // Air and film collapse as the stranded bubbles drain on exposed sand.
            stocks = vec4(stocks.xyz * exp(-dt / 1.8), stocks.w);
        }
    }
    parcels[foam_index(0u, level, local)] = Parcel(stocks, movement);
}

fn tent(distance: f32) -> f32 { return max(0.0, 1.0 - abs(distance)); }
fn kernel(distance: f32, diffusion: f32) -> f32 {
    return (1.0 - 2.0 * diffusion) * tent(distance)
        + diffusion * (tent(distance - 1.0) + tent(distance + 1.0));
}

fn carry_coarse_handoff(local: vec2<i32>, relative: vec2<i32>) {
    // Keep the previous window's feather as it moves into the interior.
    // New cells start at the distant estimate. Their contribution grows as
    // the existing shore births and transport build up the local stocks.
    var deficit = coarse_deficit(foam_settings.banks.x, relative);
    if any(relative != local) {
        deficit = max(deficit, 1.0 - foam_edge(vec2<f32>(relative) + 0.5, 1u));
    }
    let fill = foam_settings.time_wind.x / 8.0;
    foam_stocks.coarse_deficit[coarse_deficit_index(foam_settings.banks.y, local)] = max(0.0, deficit - fill);
}

@compute @workgroup_size(8, 8)
fn step_foam(@builtin(global_invocation_id) id: vec3<u32>) {
    let local = vec2<i32>(id.xy);
    let level = id.z;
    let relative = local + foam_settings.origins[2u + level].xy - foam_settings.origins[level].xy;
    let radius = select(2, 3, level == 0u && foam_settings.time_wind.x > 0.024);
    var sum = vec4(0.0);
    for (var z = -radius; z <= radius; z++) {
        for (var x = -radius; x <= radius; x++) {
            let at = relative + vec2(x, z);
            if !foam_inside(at) { continue; }
            let source = parcels[foam_index(0u, level, at)];
            let distance = vec2<f32>(f32(x), f32(z)) + source.movement.xy;
            let weight = kernel(distance.x, source.movement.z) * kernel(distance.y, source.movement.z);
            sum += weight * source.stocks;
        }
    }
    let film = sum.yz / max(1.0, sum.y + sum.z);
    foam_stocks.stocks[foam_index(foam_settings.banks.y, level, local)] = clamp(vec4(sum.x, film, sum.w), vec4(0.0), vec4(1.0));
    if FOAM_HANDOFF && level == 1u {
        carry_coarse_handoff(local, relative);
    }
}

// Beyond the coarse level the surface continues the cover the level holds.
// Measure the open sea's mean stocks in a ring inside the level's edge,
// away from the cells that lose inflow at the window's edge, so the far
// estimator brings the same mean across the blend.
const HELD_INNER: i32 = 16;
const HELD_OUTER: i32 = 64;
var<workgroup> held_sums: array<vec4<f32>, 256>;

@compute @workgroup_size(256)
fn measure_held(@builtin(local_invocation_index) lane: u32) {
    var sum = vec4(0.0);
    let cells = u32(FOAM_SIDE * FOAM_SIDE);
    for (var index = lane; index < cells; index += 256u) {
        let at = vec2<i32>(i32(index % u32(FOAM_SIDE)), i32(index / u32(FOAM_SIDE)));
        let edge = min(min(at.x, at.y), min(FOAM_SIDE - 1 - at.x, FOAM_SIDE - 1 - at.y));
        if edge < HELD_INNER || edge >= HELD_OUTER { continue; }
        let parcel = parcels[foam_index(0u, 1u, at)];
        // Dry cells take no step, and surf has its own far estimator.
        if parcel.movement.z <= 0.0 || parcel.movement.w > 0.001 { continue; }
        sum += vec4(parcel.stocks.xyz, 1.0);
    }
    held_sums[lane] = sum;
    workgroupBarrier();
    for (var stride = 128u; stride > 0u; stride >>= 1u) {
        if lane < stride { held_sums[lane] += held_sums[lane + stride]; }
        workgroupBarrier();
    }
    if lane == 0u {
        let total = held_sums[0];
        let held = select(vec4(0.0), vec4(total.xyz / max(total.w, 1.0), 1.0), total.w >= 64.0);
        foam_stocks.stocks[foam_held_index(foam_settings.banks.y)] = held;
    }
}
