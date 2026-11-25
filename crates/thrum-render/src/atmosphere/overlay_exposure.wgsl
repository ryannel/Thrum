@group(0) @binding(5) var<uniform> previous_exposure: ExposureSample;
const WORLD_LIGHT_ADAPTING: u32 = 32u;

fn adapt_world_light_ev(raw_ev: f32, incident_lux: f32, limits: vec2<f32>) -> vec2<f32> {
    // Use regional sunlight for camera gain so a shadow at the metering
    // point does not brighten the still-sunlit valley. Keep the local lux
    // as the incident reading and the authority for the aerial scale.
    let clear = environment_light.sky_diffuse[2].w;
    var daylight_ratio = 1.0;
    var display_ev = raw_ev;
    if clear > 1e-8 && incident_lux > 1e-8 {
        let diffuse = dot(environment_light.horizontal_irradiance.rgb, REC709_LUMA);
        let direct = dot(directional.irradiance.rgb, REC709_LUMA)
            * max(directional.direction_to_light.y, 0.0);
        let regional = diffuse + direct * world_light_regional_sun();
        daylight_ratio = clamp(regional / clear, 0.00001, 1.0);
        if abs(regional - clear) > clear * 1e-6 || abs(incident_lux - clear) > clear * 1e-6 {
            let daylight_ev = log2(clear / incident_lux);
            display_ev += daylight_ev + 0.55 * log2(daylight_ratio);
        }
    }
    let desired_ev = clamp(display_ev, limits.x, limits.y);
    let previous = vec2(serials.previous_lo, serials.previous_hi);
    let was_adapting = (previous_exposure.metadata.x & WORLD_LIGHT_ADAPTING) != 0u;
    if all(previous == vec2(0u)) ||
        any(previous != previous_exposure.metadata.yz) ||
        (previous_exposure.metadata.x & NOT_METERED) != 0u {
        return vec2(desired_ev, select(0.0, 1.0, daylight_ratio < 1.0));
    }
    let change = desired_ev - previous_exposure.values.z;
    if daylight_ratio == 1.0 && (abs(change) <= 0.0005 || (!was_adapting && abs(change) <= 0.025)) {
        return vec2(desired_ev, 0.0);
    }
    // Ease display gain only. Keep the current storage scale and the measured
    // light apart from adaptation. A camera cut takes the new reading directly.
    let delta = clamp(change * 0.10, -0.025, 0.025);
    return vec2(previous_exposure.values.z + delta, 1.0);
}
