// Reverse-Z min depth pyramid for two-phase occlusion (G3c design, "Pyramid").
// Mip 0 is previousPow2 of the depth target per axis; each texel takes the min
// (the farthest depth) over its whole source footprint, up to 3x3 at
// non-integer ratios. Later mips take 2x2 mins, and odd edges take the extra
// row and column. One dispatch per mip with portable storage writes; the CPU
// reference is visibility/occlusion_reference.rs.

@group(0) @binding(0) var depth_source: texture_depth_2d;
@group(0) @binding(1) var mip0: texture_storage_2d<r32float, write>;

@compute @workgroup_size(8, 8)
fn pyramid_mip0(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(mip0);
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let source = textureDimensions(depth_source);
    let low = (id.xy * source) / size;
    let high = min(((id.xy + 1u) * source + size - 1u) / size, source);
    var farthest = 1.0;
    for (var y = low.y; y < high.y; y++) {
        for (var x = low.x; x < high.x; x++) {
            farthest = min(farthest, textureLoad(depth_source, vec2(x, y), 0));
        }
    }
    textureStore(mip0, id.xy, vec4(farthest));
}

@group(0) @binding(0) var previous: texture_2d<f32>;
@group(0) @binding(1) var next: texture_storage_2d<r32float, write>;

@compute @workgroup_size(8, 8)
fn pyramid_down(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(next);
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let source = textureDimensions(previous);
    let low = min(id.xy * 2u, source - 1u);
    // The last texel on an odd axis also takes the extra row or column.
    let last = vec2<u32>(id.xy + 1u == size);
    let high = min(id.xy * 2u + 2u + (source & vec2(1u)) * last, source);
    var farthest = 1.0;
    for (var y = low.y; y < high.y; y++) {
        for (var x = low.x; x < high.x; x++) {
            farthest = min(farthest, textureLoad(previous, vec2(x, y), 0).r);
        }
    }
    textureStore(next, id.xy, vec4(farthest));
}
