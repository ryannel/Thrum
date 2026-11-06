// Flat horizontal incident meter: C=250 lux at ISO100. Camera normalization
// e=2^-EV100/1.2 follows Filament's physically based camera convention.
struct Policy {
    inverse_compensation_min_max: vec4<f32>,
    mode: u32,
    pre_exposure: f32,
    atmosphere: u32,
    sdr_tone_map: u32,
}
struct Serials {
    previous: vec2<u32>,
    current: vec2<u32>,
    alpha_padding: vec4<f32>,
}
struct ExposureSample {
    // Storage p, inverse camera gain, resolved camera EV100, actual incident lux.
    values: vec4<f32>,
    // Flags, current submitted-candidate serial lo/hi, SDR tone-map selector.
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
    if policy.mode == 1u {
        flags = INVALID | NOT_METERED;
        if policy.atmosphere != 0u && finite(directional[0].y)
            && valid_rgb(directional[1].rgb) && valid_rgb(directional[3].rgb) {
            let incident = directional[3].rgb + directional[1].rgb * max(directional[0].y, 0.0);
            let measured = dot(incident, vec3(0.2126,0.7152,0.0722));
            if finite(measured) && measured >= 0.0 {
                lux = measured;
                // The compensation-dependent floor reaches exactly the selected
                // minimum camera EV, without inventing physical incident light.
                let floor_lux = 2.5 * exp2(parameters.z + parameters.y);
                let raw_ev = log2(max(measured, floor_lux) * 0.4) - parameters.y;
                ev = clamp(raw_ev, parameters.z, parameters.w);
                inverse = 1.2 * exp2(ev);
                flags = 0u;
                if measured < floor_lux { flags |= FLOOR | CLAMP; }
                if raw_ev > parameters.w { flags |= CLAMP; }
            }
        }
    }
    result.values = vec4(policy.pre_exposure, inverse, ev, lux);
    result.metadata = vec4(flags, serials.current, policy.sdr_tone_map);
}
