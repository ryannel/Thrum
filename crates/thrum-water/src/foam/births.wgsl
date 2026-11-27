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
                let contact = 1.0 + 0.85 * exp(-3.2 * behind);
                let birth = min(3.3, 1.45 * flow * flow) * core * contact;
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
