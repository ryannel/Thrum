@group(0) @binding(5) var<uniform> previous_exposure: ExposureSample;
const WORLD_LIGHT_ADAPTING: u32 = 32u;

fn adapt_world_light_ev(raw_ev: f32, lux: f32, limits: vec2<f32>) -> vec2<f32> {
    // Meter display gain against an open horizontal receiver with the
    // current sky diffuse. A passing local shadow cannot brighten the
    // sunlit slopes. Lift gain only when the deck broadly closes.
    let clear = environment_light.sky_diffuse[3].w;
    let closure = smoothstep(0.88, 0.99, environment_light.sky_diffuse[4].w);
    var desired_ev = raw_ev;
    if clear > 1e-8 && lux > 1e-8 {
        let direct = dot(directional.irradiance.rgb, REC709_LUMA)
            * max(0.0, directional.direction_to_light.y);
        let diffuse = dot(environment_light.horizontal_irradiance.rgb, REC709_LUMA);
        let open = max(clear, direct + diffuse);
        let display_lux = mix(open, clear, closure);
        desired_ev += log2(display_lux / lux) - 0.48 * closure;
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
