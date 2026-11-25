@compute @workgroup_size(SOFT_SUN_TILE, SOFT_SUN_TILE)
fn trace_sun_and_ambient(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= frame.extent) { return; }
    let pixel = vec2<i32>(id.xy);
    let receiver = visibility_receiver(pixel, frame.extent, frame.inverse);
    trace_sun_pixel(pixel, receiver);

    let texel = pixel / i32(ambient_frame.stride);
    if all(pixel == ambient_sample_pixel(texel, ambient_frame)) {
        trace_ambient_pixel(texel, pixel, receiver);
    }
}
