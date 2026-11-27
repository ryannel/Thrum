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
