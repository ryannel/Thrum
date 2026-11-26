struct FoamHull {
    location_axis: vec4<f32>,
    extent_rate: vec4<f32>,
    velocity: vec4<f32>,
}
struct FoamSettings {
    origins: array<vec4<i32>, 4>,
    banks: vec4<u32>,
    time_wind: vec4<f32>,
    camera: vec4<f32>,
    jacobian_variance: vec4<f32>,
    long_bounds: vec4<f32>,
    hulls: array<FoamHull, HULL_LIMIT>,
    components: array<Component, COMPONENT_LIMIT>,
}
struct FoamEndpoints {
    stocks: array<vec4<f32>, FOAM_STOCK_RECORDS>,
    coarse_deficit: array<f32, FOAM_HANDOFF_CELLS>,
}
@group(3) @binding(9) var<storage, read> foam_stocks: FoamEndpoints;
@group(3) @binding(10) var<uniform> foam_settings: FoamSettings;

fn column_component(index: u32) -> Component { return foam_settings.components[index]; }

fn foam_column(point: vec2<f32>, gains: vec4<f32>, dt: f32, current: vec2<f32>, track: vec4<f32>) -> Wave {
    var field = long_wave(point - foam_settings.camera.xy, dt, gains.x,
        foam_settings.camera.w, foam_settings.banks.w);
    let slope = world_slope(field);
    field.velocity.y -= dot(slope, field.velocity.xz);
    field.slope = slope;
    let motion = cascade_motion(point, current, track, water.carriage.w, water.carriage.y);
    for (var band = 0u; band < 3u; band++) {
        let gain = gains[band + 1u];
        if gain * water.heights[band + 1u] <= 0.0 { continue; }
        let carried = carried_column(motion, water.lengths[band], band, 0.0, vec2(1.0));
        let sample = carried.wave;
        let band_slope = cascade_world_slope(sample.slope, carried.differential, gain);
        let velocity = gain * (sample.velocity.y + carried.advection_rate);
        field.displacement.y += gain * sample.displacement.y + dt * velocity;
        field.slope += band_slope;
        field.velocity.y += velocity;
    }
    return field;
}
fn foam_inside(at: vec2<i32>) -> bool {
    return all(at >= vec2(0)) && all(at < vec2(FOAM_SIDE));
}
fn foam_index(bank: u32, level: u32, at: vec2<i32>) -> u32 {
    return (2u * bank + level) * u32(FOAM_SIDE * FOAM_SIDE) + u32(at.y * FOAM_SIDE + at.x);
}
fn foam_load(bank: u32, level: u32, at: vec2<i32>) -> vec4<f32> {
    if !foam_inside(at) { return vec4(0.0); }
    return foam_stocks.stocks[foam_index(bank, level, at)];
}
fn foam_level(point: vec2<f32>, endpoint: u32, level: u32) -> vec4<f32> {
    let coordinate = point / FOAM_SPACING[level] - vec2<f32>(foam_settings.origins[2u * endpoint + level].xy);
    let centre = coordinate - 0.5;
    let at = vec2<i32>(floor(centre));
    let bank = foam_settings.banks[endpoint];
    let f = fract(centre);
    return mix(
        mix(foam_load(bank, level, at), foam_load(bank, level, at + vec2(1, 0)), f.x),
        mix(foam_load(bank, level, at + vec2(0, 1)), foam_load(bank, level, at + vec2(1, 1)), f.x),
        f.y);
}
fn foam_edge(p: vec2<f32>, level: u32) -> f32 {
    let edge = min(p, vec2(f32(FOAM_SIDE)) - p);
    // The coarse level hands over to the far estimator over 80 m, which
    // spans many rows even from high above the sea.
    return smoothstep(1.0, select(14.0, 40.0, level == 1u), min(edge.x, edge.y));
}

fn foam_edge_at(point: vec2<f32>, endpoint: u32, level: u32) -> f32 {
    return foam_edge(point / FOAM_SPACING[level] -
        vec2<f32>(foam_settings.origins[2u * endpoint + level].xy), level);
}

fn coarse_deficit_index(bank: u32, at: vec2<i32>) -> u32 {
    return bank * u32(FOAM_SIDE * FOAM_SIDE) + u32(at.y * FOAM_SIDE + at.x);
}
fn coarse_deficit(bank: u32, at: vec2<i32>) -> f32 {
    if !foam_inside(at) { return 1.0; }
    return foam_stocks.coarse_deficit[coarse_deficit_index(bank, at)];
}
fn coarse_support(point: vec2<f32>, endpoint: u32) -> f32 {
    let edge = foam_edge_at(point, endpoint, 1u);
    if !FOAM_HANDOFF || edge == 0.0 { return edge; }
    let centre = point / FOAM_SPACING[1] - vec2<f32>(foam_settings.origins[2u * endpoint + 1u].xy) - 0.5;
    let at = vec2<i32>(floor(centre));
    let f = fract(centre);
    let bank = foam_settings.banks[endpoint];
    let deficit = mix(
        mix(coarse_deficit(bank, at), coarse_deficit(bank, at + vec2(1, 0)), f.x),
        mix(coarse_deficit(bank, at + vec2(0, 1)), coarse_deficit(bank, at + vec2(1, 1)), f.x), f.y);
    return min(edge, 1.0 - deficit);
}
// The mean open-sea stocks the coarse level held at each endpoint, with a
// weight of one once measured. They follow the two banks of cells.
fn foam_held_index(bank: u32) -> u32 {
    return 4u * u32(FOAM_SIDE * FOAM_SIDE) + bank;
}
fn foam_held() -> vec4<f32> {
    return mix(foam_stocks.stocks[foam_held_index(foam_settings.banks.x)],
        foam_stocks.stocks[foam_held_index(foam_settings.banks.y)], foam_settings.time_wind.y);
}
struct FoamSample {
    stocks: vec4<f32>,
    distant_weight: f32,
}
fn foam_endpoint(point: vec2<f32>, endpoint: u32, far: vec4<f32>) -> FoamSample {
    let fine_weight = foam_edge_at(point, endpoint, 0u);
    if fine_weight >= 1.0 { return FoamSample(foam_level(point, endpoint, 0u), 0.0); }
    let coarse_weight = coarse_support(point, endpoint);
    var coarse = far;
    if coarse_weight > 0.0 {
        coarse = mix(far, foam_level(point, endpoint, 1u), coarse_weight);
    }
    let distant_weight = (1.0 - coarse_weight) * (1.0 - fine_weight);
    if fine_weight <= 0.0 { return FoamSample(coarse, distant_weight); }
    return FoamSample(mix(coarse, foam_level(point, endpoint, 0u), fine_weight), distant_weight);
}
fn foam_at(point: vec2<f32>, far: vec4<f32>) -> FoamSample {
    let a = foam_endpoint(point, 0u, far);
    let b = foam_endpoint(point, 1u, far);
    let alpha = foam_settings.time_wind.y;
    return FoamSample(mix(a.stocks, b.stocks, alpha), mix(a.distant_weight, b.distant_weight, alpha));
}
