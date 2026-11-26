@group(1) @binding(0) var opaque_depth: texture_depth_2d;
@group(1) @binding(2) var opaque_colour: texture_2d<f32>;
@group(1) @binding(3) var captured_radiance: texture_storage_2d<rgba16float, write>;
@group(1) @binding(4) var captured_distance: texture_storage_2d<r32float, write>;

@compute @workgroup_size(8, 8)
fn gather(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= optics.dispatch.zw) { return; }
    let pixel = optics.dispatch.xy + id.xy;
    if !optics_source(pixel) { return; }
    let address = vec2<i32>(pixel);
    let raw_depth = textureLoad(opaque_depth, address, 0);
    var radiance = vec3(0.0);
    var distance = 0.0;
    if raw_depth > 0.0 {
        let uv = optics_uv(pixel);
        let homogeneous = optics_unproject(uv, raw_depth);
        let world = homogeneous.xyz / homogeneous.w;
        let path = length(world - view.camera_xyz_time.xyz);
        if path <= 4000.0 {
            let air = environment_aerial(world, uv);
            let scale = environment_storage_scale();
            let colour = textureLoad(opaque_colour, address, 0).rgb;
            radiance = max(colour - air.scatter * scale, vec3(0.0)) /
                max(air.transmission, vec3(0.001));
            distance = path;
        }
    }
    textureStore(captured_radiance, address, vec4(min(radiance, vec3(65504.0)), 0.0));
    textureStore(captured_distance, address, vec4(distance, 0.0, 0.0, 0.0));
}
