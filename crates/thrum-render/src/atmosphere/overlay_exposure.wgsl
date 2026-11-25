@group(0) @binding(5) var<uniform> previous_exposure: ExposureSample;

fn adapt_world_light_ev(raw_ev: f32, limits: vec2<f32>) -> f32 {
    // Preserve part of the change in daylight that a horizontal incident
    // meter would compensate away. The stored incident lux remains physical.
    let ratio = clamp(environment_light.sky_diffuse[1].w, 0.25, 1.0);
    let desired_ev = clamp(raw_ev - 0.45 * log2(ratio), limits.x, limits.y);
    let previous = vec2(serials.previous_lo, serials.previous_hi);
    if ratio == 1.0 || all(previous == vec2(0u)) ||
        any(previous != previous_exposure.metadata.yz) ||
        (previous_exposure.metadata.x & NOT_METERED) != 0u {
        return desired_ev;
    }
    // Ease display gain only. Keep the current storage scale and the measured
    // light apart from adaptation. A camera cut takes the new reading directly.
    let delta = clamp((desired_ev - previous_exposure.values.z) * 0.14, -0.055, 0.055);
    return previous_exposure.values.z + delta;
}
