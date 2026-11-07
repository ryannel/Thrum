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
// comparison. The footprint's x and y carry no explicit term: a pixel is
// covered only when its centre is inside, and the texels holding the corners
// reach at least half a pixel past them, so x / y rounding below half a pixel
// stays inside the read rectangle. The worst-case bound exceeds that only
// for w below a few metres at the field's far extent (M3c widens or proves
// it with mesh faces).

// gamma16 = 16 u / (1 - 16 u), u = 2^-24.
const CLIP_GAMMA: f32 = 9.5367522e-7;
const DEPTH_ULPS: f32 = 10.0;

fn depth_ulp(depth: f32) -> f32 {
    let magnitude = abs(depth);
    return bitcast<f32>(bitcast<u32>(magnitude) + 1u) - magnitude;
}

fn occluded_by_pyramid(corners: array<vec4<f32>, 8>, magnitude: vec2<f32>) -> bool {
    let size = textureDimensions(pyramid, 0);
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
        nearest = max(nearest, corner.z / corner.w);
        w_min = min(w_min, corner.w);
        let uv = vec2(0.5 + 0.5 * corner.x / corner.w, 0.5 - 0.5 * corner.y / corner.w);
        let texel = vec2<u32>(clamp(floor(uv * vec2<f32>(size)), vec2(0.0), vec2<f32>(size - 1u)));
        low = min(low, texel);
        high = max(high, texel);
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
    let w_low = w_min - 2.0 * CLIP_GAMMA * magnitude.y;
    if w_low <= 0.0 {
        return false;
    }
    let slack = 2.0 * CLIP_GAMMA * (magnitude.x + abs(nearest) * magnitude.y) / w_low
        + DEPTH_ULPS * depth_ulp(nearest);
    return nearest + slack < farthest;
}
