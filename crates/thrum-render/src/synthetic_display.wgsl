// The target is Rgba8Unorm: this shader performs the sole sRGB encoding.
// This pre-AA diagnostic loads one exact fp16 texel, with no filtering/sampler.
@group(0) @binding(2) var motion: texture_2d<f32>;
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var<uniform> exposure: ExposureSample;
fn fullscreen_position(index: u32) -> vec4<f32> {
    let positions = array<vec2<f32>, 3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(positions[index],0.0,1.0);
}
@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return fullscreen_position(index);
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
    let denominator = max(reciprocal_scale+dot(scaled,REC709_LUMA),max(scaled.r,max(scaled.g,scaled.b)));
    // A black scene has zero denominator only when a subnormal reciprocal flushes.
    return scaled/max(denominator,1e-30);
}

@group(1) @binding(0) var<uniform> edr_headroom: vec2<f32>;

struct EdrVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex fn edr_vertex(@builtin(vertex_index) index: u32) -> EdrVertex {
    let position = fullscreen_position(index);
    return EdrVertex(position, vec2(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5));
}

@fragment fn edr_fragment(in: EdrVertex) -> @location(0) vec4<f32> {
    let size = textureDimensions(scene);
    let pixel = clamp(vec2<i32>(in.uv * vec2<f32>(size)), vec2<i32>(0), vec2<i32>(size) - 1);
    let stored = textureLoad(scene, pixel, 0).rgb;
    let overlay = exposure.metadata.w >> DISPLAY_OVERLAY_SHIFT;
    if overlay == OVERLAY_MOTION || overlay == OVERLAY_EXPOSURE_ZONES || overlay == OVERLAY_TEMPORAL_INPUTS {
        let encoded = clamp(sdr_pixel(vec4(vec2<f32>(pixel), 0.0, 1.0)).rgb, vec3(0.0), vec3(1.0));
        let low = encoded / SRGB_SLOPE;
        let high = pow((encoded + vec3(SRGB_OFFSET)) / SRGB_SCALE, vec3(SRGB_GAMMA));
        return vec4(select(high, low, encoded <= vec3(SRGB_ENCODED_KNEE)), 1.0);
    }
    let mapped = neutral_highlights(stored, exposure.values.x * exposure.values.y, edr_headroom.x, true);
    // A drawable rounded to binary16 must still respect the current f32 limit.
    return vec4(min(mapped, vec3(edr_headroom.y)), 1.0);
}
// Exposed luminance of an 18% Lambertian card lit at the metered illuminance with zero
// compensation: 0.18 / (pi * 1.2 * 0.4), from the incident meter's C=250 and e=1/(1.2*2^EV100).
const ZONE_REFERENCE: f32 = 0.119366207;
// One-stop false-colour bands from -6 (and darker) to +6 (and brighter), as exact display
// codes: purple and blue below, grey at the reference, yellow, red and pink above, white at +6.
fn exposure_zone(stored: vec3<f32>, storage_inverse_gain: f32) -> vec3<f32> {
    var palette = array<vec3<f32>, 13>(
        vec3(20.0, 0.0, 40.0), vec3(90.0, 20.0, 140.0), vec3(40.0, 60.0, 200.0),
        vec3(20.0, 140.0, 230.0), vec3(20.0, 190.0, 180.0), vec3(90.0, 170.0, 90.0),
        vec3(128.0, 128.0, 128.0),
        vec3(190.0, 220.0, 70.0), vec3(250.0, 220.0, 40.0), vec3(250.0, 140.0, 20.0),
        vec3(230.0, 40.0, 30.0), vec3(250.0, 110.0, 200.0), vec3(255.0, 255.0, 255.0),
    );
    let luminance = dot(stored, REC709_LUMA);
    var zone = 0;
    if luminance > 0.0 {
        // Two divisions keep a tiny gain from underflowing a product; overflow saturates.
        let stops = log2(luminance / storage_inverse_gain / ZONE_REFERENCE);
        zone = i32(clamp(round(stops), -6.0, 6.0)) + 6;
    }
    return palette[zone] / 255.0;
}
fn sdr_pixel(position: vec4<f32>) -> vec4<f32> {
    if (exposure.metadata.w >> DISPLAY_OVERLAY_SHIFT) == OVERLAY_MOTION {
        let motion_extent = textureDimensions(motion);
        let colour_extent = textureDimensions(scene);
        var motion_pixel = vec2<i32>(position.xy);
        if any(motion_extent != colour_extent) {
            motion_pixel = vec2<i32>(position.xy * vec2<f32>(motion_extent) / vec2<f32>(colour_extent));
        }
        let velocity = textureLoad(motion, motion_pixel, 0).xy;
        if any(abs(velocity) >= vec2(MAX_FINITE_HALF)) { return vec4(1.0, 0.0, 1.0, 1.0); }
        return vec4(clamp(vec2(0.5) + velocity / 16.0, vec2(0.0), vec2(1.0)),
            clamp(length(velocity) / 16.0, 0.0, 1.0), 1.0);
    }
    let stored = textureLoad(scene,vec2<i32>(position.xy),0).rgb;
    let selectors = exposure.metadata.w;
    if (selectors >> DISPLAY_OVERLAY_SHIFT) == OVERLAY_TEMPORAL_INPUTS { return vec4(stored, 1.0); }
    // The overlay reads the exposed scene before any tone scale and is written undithered.
    if (selectors >> DISPLAY_OVERLAY_SHIFT) == OVERLAY_EXPOSURE_ZONES {
        return vec4(exposure_zone(stored, exposure.values.x * exposure.values.y), 1.0);
    }
    var mapped: vec3<f32>;
    if (selectors & TONE_MAP_MASK) == TONE_MAP_NEUTRAL_HIGHLIGHTS {
        mapped = neutral_highlights(stored, exposure.values.x * exposure.values.y, 1.0, false);
    } else {
        mapped = reinhard(stored);
    }
    let encoded = srgb_encode(mapped);
    if (selectors >> DISPLAY_OVERLAY_SHIFT) == OVERLAY_UNDITHERED { return vec4(encoded, 1.0); }
    return vec4(dither_unorm8(encoded, vec2<u32>(position.xy)),1.0);
}

@fragment fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return sdr_pixel(position);
}
