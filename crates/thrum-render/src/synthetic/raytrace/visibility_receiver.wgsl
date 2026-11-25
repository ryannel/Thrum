fn world_at(pixel: vec2<i32>, z: f32, extent: vec2<u32>, inverse: mat4x4<f32>) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + 0.5) / vec2<f32>(extent);
    let h = inverse * vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), z, 1.0);
    return h.xyz / h.w;
}

fn plane_tangent(pixel: vec2<i32>, axis: vec2<i32>, z: f32, point: vec3<f32>,
    extent: vec2<u32>, inverse: mat4x4<f32>) -> vec3<f32> {
    let bounds = vec2<i32>(extent) - 1;
    let lo = max(pixel - axis, vec2(0));
    let hi = min(pixel + axis, bounds);
    let left = textureLoad(depth, lo, 0);
    let right = textureLoad(depth, hi, 0);
    if right > 0.0 && any(hi != pixel) &&
        (left <= 0.0 || all(lo == pixel) || abs(right - z) < abs(left - z)) {
        return world_at(hi, right, extent, inverse) - point;
    }
    if left > 0.0 && any(lo != pixel) { return point - world_at(lo, left, extent, inverse); }
    return world_at(pixel + axis, z, extent, inverse) - point;
}

fn hash(seed: u32) -> u32 {
    var x = seed;
    x = (x ^ (x >> 16u)) * 0x7feb352du;
    x = (x ^ (x >> 15u)) * 0x846ca68bu;
    return x ^ (x >> 16u);
}

// Rebuild the camera-relative point and receiver plane once for both effects.
struct VisibilityReceiver {
    point: vec3<f32>,
    normal: vec3<f32>,
    z: f32,
}

fn visibility_receiver(pixel: vec2<i32>, extent: vec2<u32>, inverse: mat4x4<f32>) -> VisibilityReceiver {
    let z = textureLoad(depth, pixel, 0);
    if z <= 0.0 { return VisibilityReceiver(vec3(0.0), vec3(0.0), 0.0); }
    let point = world_at(pixel, z, extent, inverse);
    let tangent = plane_tangent(pixel, vec2(1, 0), z, point, extent, inverse);
    let bitangent = plane_tangent(pixel, vec2(0, 1), z, point, extent, inverse);
    return VisibilityReceiver(point, cross(bitangent, tangent), z);
}
