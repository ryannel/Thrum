@group(0) @binding(0) var<uniform> frame: AmbientFrame;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(5) var current_samples: texture_2d<u32>;
@group(0) @binding(6) var filtered_out: texture_storage_2d<rg32float, write>;

@compute @workgroup_size(AMBIENT_TILE, AMBIENT_TILE)
fn trace_ambient(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= ambient_extent(ambient_frame)) { return; }
    let texel = vec2<i32>(id.xy);
    let pixel = ambient_sample_pixel(texel, ambient_frame);
    trace_ambient_pixel(texel, pixel, visibility_receiver(pixel, ambient_frame.extent, ambient_frame.inverse));
}

@compute @workgroup_size(AMBIENT_TILE, AMBIENT_TILE)
fn filter_ambient(@builtin(global_invocation_id) id: vec3<u32>) {
    let extent = ambient_extent(frame);
    if any(id.xy >= extent) { return; }
    let texel = vec2<i32>(id.xy);
    let centre = textureLoad(current_samples, texel, 0);
    let z = bitcast<f32>(centre.z);
    if centre.y == 0u {
        textureStore(filtered_out, texel, vec4(1.0, 0.0, 0.0, 0.0));
        return;
    }

    let pixel = ambient_sample_pixel(texel, frame);
    let point = world_at(pixel, z, frame.extent, frame.inverse);
    let normal = normalize(unpack4x8snorm(centre.w).xyz);
    let footprint = max(length(point - world_at(pixel + vec2(1, 0), z, frame.extent, frame.inverse)), 1e-4);
    // Evaluate the centre plane in clip space. Taps only need a dot
    // product with the screen/depth delta instead of another unprojection.
    let plane = vec4(normal, -dot(normal, point));
    let screen_scale = vec2(2.0, -2.0) / vec2<f32>(frame.extent);
    let plane_delta = vec3(dot(plane, frame.inverse[0]) * screen_scale.x,
        dot(plane, frame.inverse[1]) * screen_scale.y, dot(plane, frame.inverse[2]));
    let w_delta = vec3(frame.inverse[0].w * screen_scale.x,
        frame.inverse[1].w * screen_scale.y, frame.inverse[2].w);
    let uv = (vec2<f32>(pixel) + 0.5) / vec2<f32>(frame.extent);
    let centre_w = (frame.inverse * vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), z, 1.0)).w;
    let kernel = array(1.0, 0.93, 0.75, 0.53, 0.33);
    var occlusion = 0.0;
    var total = 0.0;
    for (var y = -4; y <= 4; y++) {
        for (var x = -4; x <= 4; x++) {
            let at = texel + vec2(x, y);
            if !ambient_in_bounds(at, extent) { continue; }
            let sample = textureLoad(current_samples, at, 0);
            if sample.y == 0u { continue; }
            let delta = vec3(vec2<f32>(ambient_sample_pixel(at, frame) - pixel), bitcast<f32>(sample.z) - z);
            let distance = abs(dot(plane_delta, delta) / (centre_w + dot(w_delta, delta)));
            let same_plane = 1.0 - smoothstep(0.35, 1.35, distance / footprint);
            let aligned = smoothstep(0.85, 0.99, dot(normal, unpack4x8snorm(sample.w).xyz));
            let weight = aligned * same_plane * kernel[abs(x)] * kernel[abs(y)];
            occlusion += weight * f32(sample.y - countOneBits(sample.x)) / f32(sample.y);
            total += weight;
        }
    }
    textureStore(filtered_out, texel, vec4(select(1.0, 1.0 - occlusion / total, total > 0.0), z, 0.0, 0.0));
}
