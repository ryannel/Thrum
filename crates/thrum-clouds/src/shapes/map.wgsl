struct PlacedWindows {
    level: array<vec4<f32>, 4>,
}
@group(0) @binding(0) var<uniform> placed_windows: PlacedWindows;
@group(0) @binding(1) var placed_map: texture_storage_2d_array<rgba32float, write>;

@compute @workgroup_size(8, 8)
fn write_placed_map(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(placed_map)) { return; }
    let window = placed_windows.level[id.z];
    let point = window.xy + (vec2<f32>(id.xy) + vec2(0.5)) * window.z;
    var result = vec4(0.0);
    if placed_inside_bounds(point) {
        let candidates = placed_candidates(point);
        for (var index = candidates.x; index < candidates.y; index++) {
            let shape = placed_instances.items[index];
            let uv = placed_uv(point, shape);
            let column = placed_column(shape, uv);
            if column.x > 0.0 {
                let optical = placed_instances.optical;
                let tau = column.x * shape.form.x * shape.form.y * optical.x;
                result = vec4(f32(index + 1u), shape.form.z, tau, shape.form.y);
                break;
            }
        }
    }
    textureStore(placed_map, vec2<i32>(id.xy), i32(id.z), result);
}
