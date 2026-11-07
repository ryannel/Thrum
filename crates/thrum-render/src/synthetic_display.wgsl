// The target is Rgba8Unorm: this shader performs the sole sRGB encoding.
// This pre-AA diagnostic loads one exact fp16 texel, with no filtering/sampler.
struct ExposureSample {
    values: vec4<f32>, // Storage p, inverse camera gain, EV100, actual lux.
    metadata: vec4<u32>,
}
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var<uniform> exposure: ExposureSample;
@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(positions[index],0.0,1.0);
}
fn srgb_encode(linear: vec3<f32>) -> vec3<f32> {
    return select(1.055*pow(linear,vec3(1.0/2.4))-vec3(0.055),12.92*linear,linear<=vec3(0.0031308));
}
// Integer hash (PCG 2D, Jarzynski and Olano 2020) giving two uniform values per pixel.
fn pcg2d(seed: vec2<u32>) -> vec2<f32> {
    var v = seed * 1664525u + 1013904223u;
    v.x += v.y * 1664525u; v.y += v.x * 1664525u;
    v = v ^ (v >> vec2(16u));
    v.x += v.y * 1664525u; v.y += v.x * 1664525u;
    v = v ^ (v >> vec2(16u));
    return vec2<f32>(v >> vec2(8u)) * (1.0 / 16777216.0);
}
// Zero-mean triangular dither of one 8-bit step, fixed per pixel so captures stay
// deterministic. It breaks the contours a smooth sky otherwise shows at Unorm8. The
// amplitude is zero for values that round to black or white and reaches a full step one
// step away, so those codes stay exact.
fn dither_unorm8(encoded: vec3<f32>, pixel: vec2<u32>) -> vec3<f32> {
    let noise = pcg2d(pixel);
    let tpdf = noise.x + noise.y - 1.0;
    let edge = min(encoded, vec3(1.0) - encoded) * 255.0;
    let headroom = clamp(2.0 * edge - vec3(1.0), vec3(0.0), vec3(1.0));
    return encoded + tpdf * headroom / 255.0;
}
fn reinhard(stored: vec3<f32>) -> vec3<f32> {
    // Scaling first avoids overflow for valid large gains: algebraically this is
    // C_e/(1+Y(C_e)), followed by the specified shared maximum-channel gamut map.
    let peak = max(1.0,max(stored.r,max(stored.g,stored.b)));
    let scaled = stored/peak;
    let reciprocal_scale = (exposure.values.x*exposure.values.y)/peak;
    let denominator = max(reciprocal_scale+dot(scaled,vec3(0.2126,0.7152,0.0722)),max(scaled.r,max(scaled.g,scaled.b)));
    // A black scene has zero denominator only when a subnormal reciprocal flushes.
    return scaled/max(denominator,1e-30);
}
@fragment fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let stored = textureLoad(scene,vec2<i32>(position.xy),0).rgb;
    var mapped: vec3<f32>;
    if exposure.metadata.w == 1u {
        mapped = neutral_highlights(stored, exposure.values.x * exposure.values.y);
    } else {
        mapped = reinhard(stored);
    }
    return vec4(dither_unorm8(srgb_encode(mapped), vec2<u32>(position.xy)),1.0);
}
