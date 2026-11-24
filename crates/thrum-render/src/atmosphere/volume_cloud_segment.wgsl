// Project each metre of the view ray onto the provider's plane. A single
// query in a long cell can miss a whole gap, especially with the Sun low.
fn volume_cloud_projection(direction: vec3<f32>) -> f32 {
    let path = volume_cloud_path();
    // A negative scale means the provider has no eligible path this frame.
    if path.y <= 0.0 { return -1.0; }
    return length(direction.xz - direction.y * path.xz / path.y);
}

fn volume_cloud_visibility(eye: vec3<f32>, direction: vec3<f32>, projection: f32,
                           start: f32, end: f32) -> f32 {
    if projection < 0.0 { return 1.0; }
    let count = u32(clamp(ceil((end - start) * projection / 64.0), 1.0, 16.0));
    var visible = 0.0;
    for (var sample = 0u; sample < count; sample++) {
        let t = (f32(sample) + 0.5) / f32(count);
        visible += cloud_factor(eye + direction * mix(start, end, t));
    }
    return visible / f32(count);
}
