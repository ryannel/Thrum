struct SecondaryAirRay {
    origin: vec3<f32>,
    direction: vec3<f32>,
    distance: f32,
}

fn secondary_air_ray(world_position: vec3<f32>, screen_position: vec4<f32>) -> SecondaryAirRay {
    let camera = aerial_frame.camera_altitude.xyz;
    let offset = world_position - camera;
    let total = length(offset);
    var direction = offset / max(total, 1e-6);
    var start = 0.0;
    if secondary_view.options.y > 0.0 {
        let plane = secondary_view.clip_plane;
        let side = dot(plane, vec4(camera, 1.0));
        if side < 0.0 {
            // A rasterizer interpolates vertex attributes from quantized
            // triangle edges. Near the plane's horizon, a tiny vertical
            // attribute error becomes metres of crossing-distance error.
            // The pixel and its inverse raster matrix define the camera ray.
            let uv = screen_position.xy / secondary_view.extent.xy;
            let ndc = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
            let far = secondary_view.inverse_view_proj * vec4(ndc, 0.0, 1.0);
            direction = normalize(far.xyz - camera * far.w);
            start = clamp(-side / max(dot(plane.xyz, direction), 1e-20), 0.0, total);
        }
    }
    return SecondaryAirRay(camera + direction * start, direction, min(32000.0, total - start));
}
