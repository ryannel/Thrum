// Conservative occlusion test against the min depth pyramid (G3c design,
// "Test"). The including shader declares `pyramid: texture_2d<f32>` with every
// mip. Reverse-Z: a box is hidden only when its nearest depth is strictly
// farther than everything under its rectangle, by an absolute floor of
// 1e-6 + 4 ulp. Equality and any corner at or behind the near plane stay
// visible; the frustum decision is made before this test.

fn occlusion_floor(depth: f32) -> f32 {
    let magnitude = abs(depth);
    return 1.0e-6 + 4.0 * (bitcast<f32>(bitcast<u32>(magnitude) + 1u) - magnitude);
}

fn occluded_by_pyramid(corners: array<vec4<f32>, 8>) -> bool {
    let size = textureDimensions(pyramid, 0);
    let levels = textureNumLevels(pyramid);
    var nearest = -3.0e38;
    var low = vec2(0xffffffffu);
    var high = vec2(0u);
    for (var index = 0u; index < 8u; index++) {
        let corner = corners[index];
        if corner.w <= 0.0 || corner.z >= corner.w {
            return false;
        }
        nearest = max(nearest, corner.z / corner.w);
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
    return nearest < farthest - occlusion_floor(farthest);
}
