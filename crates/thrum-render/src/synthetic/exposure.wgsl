// Flat horizontal incident meter: C=250 lux at ISO100. Camera normalization
// e=2^-EV100/1.2 follows Filament's physically based camera convention.
struct Policy {
    inverse_compensation_min_max: vec4<f32>,
    mode: u32,
    pre_exposure: f32,
    atmosphere: u32,
    // SDR tone map in bits 0-7, display overlay in bits 8-15.
    display_selectors: u32,
}
@group(0) @binding(0) var<uniform> policy: Policy;
@group(0) @binding(1) var<uniform> directional: DirectionalLight;
@group(0) @binding(4) var<uniform> environment_light: EnvironmentLight;
@group(0) @binding(2) var<uniform> serials: Serials;
@group(0) @binding(3) var<storage,read_write> result: ExposureSample;
const FLOOR: u32 = 1u;
const CLAMP: u32 = 2u;
const NOT_METERED: u32 = 4u;
const INVALID: u32 = 8u;
// Informational SH risk on fixed normal probes; not a pixel-clamp count.
const SKY_SH_NEGATIVE_PROBE: u32 = 16u;
// Krawczyk, Myszkowski and Seidel (2005) key value, applied to the mid-grey
// luminance of the metered illuminance. Dimming starts below 1,000 lux.
fn key_value(lux: f32) -> f32 {
    let luminance = 0.18 * lux / 3.14159265;
    return 1.03 - 2.0 / (2.0 + log2(luminance + 1.0) * 0.30103);
}
fn finite(value: f32) -> bool {
    return (bitcast<u32>(value) & 0x7f800000u) != 0x7f800000u;
}
fn valid_rgb(value: vec3<f32>) -> bool {
    return all(value >= vec3(0.0)) && finite(value.x) && finite(value.y) && finite(value.z);
}
@compute @workgroup_size(1)
fn resolve() {
    let parameters = policy.inverse_compensation_min_max;
    var inverse = parameters.x; // Fixed mode preserves the CPU reciprocal bits.
    var ev = log2(inverse / CAMERA_EXPOSURE_NORMALIZATION);
    var lux = 0.0;
    var flags = NOT_METERED;
    var storage_scale = policy.pre_exposure;
    if policy.mode == EXPOSURE_INCIDENT || policy.mode == EXPOSURE_LOW_LIGHT {
        flags = INVALID | NOT_METERED;
        if policy.atmosphere != 0u && finite(directional.direction_to_light.y)
            && valid_rgb(directional.irradiance.rgb) && valid_rgb(environment_light.horizontal_irradiance.rgb) {
            let incident = environment_light.horizontal_irradiance.rgb + directional.irradiance.rgb * max(directional.direction_to_light.y, 0.0) * incident_sun_factor();
            let measured = dot(incident, REC709_LUMA);
            if finite(measured) && measured >= 0.0 {
                lux = measured;
                // Key the real illuminance. The positive guard only protects log2.
                var raw_ev = log2(max(measured * INCIDENT_METER_LUX_SCALE, 1e-20)) - parameters.y;
                if policy.mode == EXPOSURE_LOW_LIGHT {
                    raw_ev -= min(0.0, log2(key_value(measured) / key_value(ADAPTED_LUX)));
                }
                let storage_ev = clamp(raw_ev, parameters.z, parameters.w);
                let storage_inverse = CAMERA_EXPOSURE_NORMALIZATION * exp2(storage_ev);
                storage_scale = clamp(0.25 / storage_inverse, policy.pre_exposure, 64.0);
                if policy.mode == EXPOSURE_LOW_LIGHT {
                    // Compress the camera's gain at very low scene keys while
                    // preserving the pre-grade scale for dim HDR storage.
                    let depth = max(0.0, 1.5 - (raw_ev + parameters.y));
                    raw_ev += 0.125 * depth * (depth / (1.0 + depth));
                }
                ev = clamp(raw_ev, parameters.z, parameters.w);
                inverse = CAMERA_EXPOSURE_NORMALIZATION * exp2(ev);
                flags = 0u;
                if raw_ev < parameters.z { flags |= FLOOR | CLAMP; }
                if raw_ev > parameters.w { flags |= CLAMP; }
            }
        }
    }
    if policy.atmosphere!=0u && environment_light.horizontal_irradiance.w>1.0 { flags|=SKY_SH_NEGATIVE_PROBE; }
    result.values = vec4(storage_scale, inverse, ev, lux);
    result.metadata = vec4(flags, serials.current_lo, serials.current_hi, policy.display_selectors);
}
