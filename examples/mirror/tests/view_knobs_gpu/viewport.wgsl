@group(2) @binding(0) var colour: texture_2d<f32>;
@group(2) @binding(1) var depth: texture_depth_2d;
@group(2) @binding(2) var<storage, read_write> report: vec4<u32>;

@compute @workgroup_size(1)
fn inspect_viewport() {
    let dimensions = textureDimensions(colour);
    var inside = 0u;
    var outside = 0u;
    for (var y = 0u; y < dimensions.y; y++) {
        for (var x = 0u; x < dimensions.x; x++) {
            let xy = vec2<i32>(i32(x), i32(y));
            let covered = textureLoad(colour, xy, 0).a > 0.0 || textureLoad(depth, xy, 0) > 0.0;
            if covered {
                if all(vec2<u32>(x, y) < dimensions / 4u) {
                    inside++;
                } else {
                    outside++;
                }
            }
        }
    }
    report = vec4<u32>(dimensions, inside, outside);
}
