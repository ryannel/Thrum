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
struct Serials {
    previous: vec2<u32>,
    current: vec2<u32>,
    alpha_padding: vec4<f32>,
}
struct ExposureSample {
    // Storage p, inverse camera gain, resolved camera EV100, actual incident lux.
    values: vec4<f32>,
    // Flags, current submitted-candidate serial lo/hi, display selectors.
    metadata: vec4<u32>,
}
@group(0) @binding(0) var<uniform> policy: Policy;
@group(0) @binding(1) var<uniform> directional: array<vec4<f32>,4>;
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
const ADAPTED_LUX: f32 = 1000.0;
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
    var ev = log2(inverse / 1.2);
    var lux = 0.0;
    var flags = NOT_METERED;
    var storage_scale = policy.pre_exposure;
    if policy.mode == 1u || policy.mode == 2u {
        flags = INVALID | NOT_METERED;
        if policy.atmosphere != 0u && finite(directional[0].y)
            && valid_rgb(directional[1].rgb) && valid_rgb(directional[3].rgb) {
            let incident = directional[3].rgb + directional[1].rgb * max(directional[0].y, 0.0);
            let measured = dot(incident, vec3(0.2126,0.7152,0.0722));
            if finite(measured) && measured >= 0.0 {
                lux = measured;
                // Key the real illuminance. The positive guard only protects log2.
                var raw_ev = log2(max(measured * 0.4, 1e-20)) - parameters.y;
                if policy.mode == 2u {
                    raw_ev -= min(0.0, log2(key_value(measured) / key_value(ADAPTED_LUX)));
                }
                ev = clamp(raw_ev, parameters.z, parameters.w);
                inverse = 1.2 * exp2(ev);
                storage_scale = clamp(0.25 / inverse, policy.pre_exposure, 64.0);
                flags = 0u;
                if raw_ev < parameters.z { flags |= FLOOR | CLAMP; }
                if raw_ev > parameters.w { flags |= CLAMP; }
            }
        }
    }
    if policy.atmosphere!=0u && directional[3].w>1.0 { flags|=SKY_SH_NEGATIVE_PROBE; }
    result.values = vec4(storage_scale, inverse, ev, lux);
    result.metadata = vec4(flags, serials.current, policy.display_selectors);
}
