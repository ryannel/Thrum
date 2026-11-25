@group(0) @binding(5) var<uniform> previous_exposure: ExposureSample;
const WORLD_LIGHT_ADAPTING: u32 = 32u;

fn adapt_world_light_ev(raw_ev: f32, limits: vec2<f32>) -> vec2<f32> {
    // Preserve part of the change in daylight that a horizontal incident
    // meter would compensate away. The stored incident lux remains physical.
    let ratio = clamp(environment_light.sky_diffuse[1].w, 0.25, 1.0);
    let desired_ev = clamp(raw_ev - 0.45 * log2(ratio), limits.x, limits.y);
    let previous = vec2(serials.previous_lo, serials.previous_hi);
    let was_adapting = (previous_exposure.metadata.x & WORLD_LIGHT_ADAPTING) != 0u;
    if all(previous == vec2(0u)) ||
        any(previous != previous_exposure.metadata.yz) ||
        (previous_exposure.metadata.x & NOT_METERED) != 0u {
        return vec2(desired_ev, select(0.0, 1.0, ratio < 1.0));
    }
    let change = desired_ev - previous_exposure.values.z;
    if ratio == 1.0 && (abs(change) <= 0.0005 || (!was_adapting && abs(change) <= 0.025)) {
        return vec2(desired_ev, 0.0);
    }
    // Ease display gain only. Keep the current storage scale and the measured
    // light apart from adaptation. A camera cut takes the new reading directly.
    let delta = clamp(change * 0.10, -0.025, 0.025);
    return vec2(previous_exposure.values.z + delta, 1.0);
}
