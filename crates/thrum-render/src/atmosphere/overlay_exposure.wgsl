@group(0) @binding(5) var<uniform> previous_exposure: ExposureSample;
const WORLD_LIGHT_ADAPTING: u32 = 32u;

fn adapt_world_light_ev(raw_ev: f32, lux: f32, limits: vec2<f32>) -> vec2<f32> {
    let clear_sky = environment_light.sky_diffuse[2].w == 0.0;
    var desired_ev = raw_ev;
    let clear = environment_light.sky_diffuse[3].w;
    if !clear_sky && clear > 1e-8 && lux > 1e-8 {
        // Meter display gain against an open horizontal receiver with the
        // current sky diffuse, independently of local cloud shade.
        let direct = dot(directional.irradiance.rgb, REC709_LUMA)
            * max(0.0, directional.direction_to_light.y);
        let diffuse = dot(environment_light.horizontal_irradiance.rgb, REC709_LUMA);
        let broad_diffuse = clamp(log2((direct + diffuse) / clear), 0.0, 0.18);
        desired_ev += log2(clear / lux) + broad_diffuse;
    }
    desired_ev = clamp(desired_ev, limits.x, limits.y);
    let previous = vec2(serials.previous_lo, serials.previous_hi);
    let was_adapting = (previous_exposure.metadata.x & WORLD_LIGHT_ADAPTING) != 0u;
    if all(previous == vec2(0u)) ||
        any(previous != previous_exposure.metadata.yz) ||
        (previous_exposure.metadata.x & NOT_METERED) != 0u {
        return vec2(desired_ev, select(0.0, 1.0, !clear_sky));
    }
    let change = desired_ev - previous_exposure.values.z;
    // A transparent overlay can finish a weather handover already in progress.
    // Changes in clear-sky lighting alone always use the ordinary meter.
    if clear_sky && (!was_adapting || abs(change) <= 0.0005) {
        return vec2(desired_ev, 0.0);
    }
    // Ease display gain only. Keep the current storage scale and the measured
    // light apart from adaptation. A camera cut takes the new reading directly.
    // The current incident meter follows daylight. Keep only a bounded
    // residual for the cloud handover so a solar edit cannot retain night gain.
    let residual = clamp(-change, -0.5, 0.5);
    let delta = clamp(-residual * 0.10, -0.025, 0.025);
    return vec2(desired_ev + residual + delta, 1.0);
}
