@group(0) @binding(5) var<uniform> previous_exposure: ExposureSample;
const WORLD_LIGHT_ADAPTING: u32 = 32u;

fn adapt_world_light_ev(raw_ev: f32, lux: f32, limits: vec2<f32>) -> vec2<f32> {
    // Put camera gain on the unclouded daylight reference. Only broad sky
    // closure lifts the gain, so a passing local shadow cannot brighten
    // distant sunlit slopes. The incident lux remains the local reading.
    let clear = environment_light.sky_diffuse[3].w;
    let closure = smoothstep(0.68, 0.98, environment_light.sky_diffuse[4].w);
    var desired_ev = raw_ev;
    if clear > 1e-8 && lux > 1e-8 {
        desired_ev += log2(clear / lux) - 0.48 * closure;
    }
    desired_ev = clamp(desired_ev, limits.x, limits.y);
    let previous = vec2(serials.previous_lo, serials.previous_hi);
    let was_adapting = (previous_exposure.metadata.x & WORLD_LIGHT_ADAPTING) != 0u;
    if all(previous == vec2(0u)) ||
        any(previous != previous_exposure.metadata.yz) ||
        (previous_exposure.metadata.x & NOT_METERED) != 0u {
        return vec2(desired_ev, select(0.0, 1.0, closure > 0.0));
    }
    let change = desired_ev - previous_exposure.values.z;
    if closure == 0.0 && (abs(change) <= 0.0005 || (!was_adapting && abs(change) <= 0.025)) {
        return vec2(desired_ev, 0.0);
    }
    // Ease display gain only. Keep the current storage scale and the measured
    // light apart from adaptation. A camera cut takes the new reading directly.
    let delta = clamp(change * 0.10, -0.025, 0.025);
    return vec2(previous_exposure.values.z + delta, 1.0);
}
