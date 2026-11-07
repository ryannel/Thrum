// Conservative occlusion test against the min depth pyramid (G3c design,
// "Test" and "Depth tolerance"). The including shader declares
// `pyramid: texture_2d<f32>` with every mip. Reverse-Z: a box is hidden only
// when its nearest corner depth, raised by a bound on rounding, is strictly
// below everything under its rectangle. Any corner at or behind the near plane
// stays visible; the frustum decision is made before this test.
//
// The bound: the rasterized depth at any covered sample is a convex
// combination of its triangle's vertex depths (snapping moves coverage and
// barycentrics together), so it exceeds the exact box maximum only by
// rounding. The vertex shader and this pass each form clip z and w with error
// at most gamma16 times `magnitude` (the caller's sums of |row| * |position|),
// which moves z / w by at most gamma16 * (Z + |d| W) / w per path. Ten ulps of
// the depth cover the two divides (2.5 ulp each in WGSL), the rasterizer's
// interpolation (four, assumed; not witnessed on cubes, see M3c) and this
// comparison.
//
// The footprint gets the same treatment. Each path moves a corner's x / w by
// at most gamma16 * (X + |x / w| W) / w, and likewise y; the divide, the
// viewport transform and the rasterizer's sub-pixel snap (at least 8 bits)
// add a few ulps and 1/512 pixel. A pixel is covered only when its centre is
// inside, and the texels holding the corners reach at least half a screen
// pixel past them (a quarter of a level-0 texel, since the level is at most
// twice the screen), so only error beyond that quarter texel widens the read
// rectangle. Typical boxes are not widened; near ones at the field's far
// extent are.

// gamma16 = 16 u / (1 - 16 u), u = 2^-24.
const CLIP_GAMMA: f32 = 9.5367522e-7;
const DEPTH_ULPS: f32 = 10.0;
// The rasterizer's vertex snap, in pixels: 8 sub-pixel bits, rounded.
const SNAP_PIXELS: f32 = 1.0 / 512.0;

fn depth_ulp(depth: f32) -> f32 {
    let magnitude = abs(depth);
    return bitcast<f32>(bitcast<u32>(magnitude) + 1u) - magnitude;
}

// How far a corner's uv may sit from where either path puts it, beyond the
// quarter texel the coverage rule already grants (see above).
fn footprint_widening(ndc: vec2<f32>, w_low: f32, magnitude: vec4<f32>, size: vec2<f32>) -> vec2<f32> {
    let clip = CLIP_GAMMA * (magnitude.xy + abs(ndc) * magnitude.w) / w_low;
    let rounding = 2.0 * vec2(depth_ulp(ndc.x), depth_ulp(ndc.y)) + 4.0 * depth_ulp(1.0);
    return max(clip + rounding + SNAP_PIXELS / size - 0.25 / size, vec2(0.0));
}

// `magnitude` bounds the |clip x|, |clip y|, |clip z| and |clip w| terms of
// every position either path forms for this box.
fn occluded_by_pyramid(corners: array<vec4<f32>, 8>, magnitude: vec4<f32>) -> bool {
    let size = textureDimensions(pyramid, 0);
    let extent = vec2<f32>(size);
    let levels = textureNumLevels(pyramid);
    var nearest = -3.0e38;
    var w_min = 3.0e38;
    var low = vec2(0xffffffffu);
    var high = vec2(0u);
    for (var index = 0u; index < 8u; index++) {
        let corner = corners[index];
        if corner.w <= 0.0 || corner.z >= corner.w {
            return false;
        }
        let corner_low = corner.w - 2.0 * CLIP_GAMMA * magnitude.w;
        if corner_low <= 0.0 {
            return false;
        }
        nearest = max(nearest, corner.z / corner.w);
        w_min = min(w_min, corner.w);
        let ndc = corner.xy / corner.w;
        let uv = vec2(0.5 + 0.5 * ndc.x, 0.5 - 0.5 * ndc.y);
        let widening = footprint_widening(ndc, corner_low, magnitude, extent);
        let top = extent - 1.0;
        low = min(low, vec2<u32>(clamp(floor((uv - widening) * extent), vec2(0.0), top)));
        high = max(high, vec2<u32>(clamp(floor((uv + widening) * extent), vec2(0.0), top)));
    }
    // The lowest level where the rectangle spans at most two texels per axis,
    // so a misaligned rectangle cannot fall outside the up-to-2x2 read.
    var level = 0u;
    loop {
        let span = (high >> vec2(level)) - (low >> vec2(level));
        if all(span <= vec2(1u)) || level + 1u >= levels {
            break;
        }
        level += 1u;
    }
    let mip = max(size >> vec2(level), vec2(1u));
    let first = low >> vec2(level);
    let last = min(high >> vec2(level), mip - 1u);
    var farthest = 3.0e38;
    for (var y = first.y; y <= last.y; y++) {
        for (var x = first.x; x <= last.x; x++) {
            farthest = min(farthest, textureLoad(pyramid, vec2(x, y), i32(level)).r);
        }
    }
    // Every corner passed the w bound above, so this is positive.
    let w_low = w_min - 2.0 * CLIP_GAMMA * magnitude.w;
    let slack = 2.0 * CLIP_GAMMA * (magnitude.z + abs(nearest) * magnitude.w) / w_low
        + DEPTH_ULPS * depth_ulp(nearest);
    return nearest + slack < farthest;
}
