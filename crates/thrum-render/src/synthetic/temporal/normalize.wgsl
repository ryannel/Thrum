@group(0) @binding(0) var<uniform> source_exposure: ExposureSample;
@group(0) @binding(1) var<storage, read_write> display_exposure: ExposureSample;
@group(0) @binding(2) var source_colour: texture_2d<f32>;
@group(0) @binding(3) var normalized_colour: texture_storage_2d<rgba16float, write>;
struct Repairs {
    high: atomic<u32>,
    low: atomic<u32>,
    nonfinite: atomic<u32>,
}
@group(0) @binding(4) var<storage, read_write> repairs: Repairs;

const STORAGE_KEY: f32 = 0.25;
const HALF_MAX: f32 = 65504.0;

fn bounded(value: f32) -> f32 {
    let bits = bitcast<u32>(value);
    if (bits & 0x7f800000u) == 0x7f800000u {
        atomicAdd(&repairs.nonfinite, 1u);
        return 0.0;
    }
    if value > HALF_MAX {
        atomicAdd(&repairs.high, 1u);
        return HALF_MAX;
    }
    if value < 0.0 {
        atomicAdd(&repairs.low, 1u);
        return 0.0;
    }
    return value;
}

@compute @workgroup_size(8, 8)
fn normalize(@builtin(global_invocation_id) index: vec3<u32>) {
    if all(index.xy == vec2(0u)) {
        display_exposure = source_exposure;
        display_exposure.values.x = STORAGE_KEY;
        display_exposure.values.y = 1.0;
    }
    if any(index.xy >= textureDimensions(source_colour)) { return; }
    let pixel = textureLoad(source_colour, vec2<i32>(index.xy), 0);
    let factor = STORAGE_KEY / (source_exposure.values.x * source_exposure.values.y);
    let scaled = pixel.rgb * factor;
    textureStore(normalized_colour, vec2<i32>(index.xy),
                 vec4(bounded(scaled.r), bounded(scaled.g), bounded(scaled.b), pixel.a));
}

@compute @workgroup_size(1)
fn twin() {
    display_exposure = source_exposure;
}
