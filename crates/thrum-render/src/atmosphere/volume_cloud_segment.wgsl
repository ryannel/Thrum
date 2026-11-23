fn volume_cloud_visibility(eye: vec3<f32>, direction: vec3<f32>, sun: vec3<f32>,
                           start: f32, end: f32) -> f32 {
    if sun.y <= 0.0 { return 1.0; }
    // Project the cell's length onto the Sun-path plane. A single query in
    // a long cell can miss a whole gap, especially with the Sun low.
    let projected = length(direction.xz - direction.y * sun.xz / max(0.02, sun.y));
    let count = u32(clamp(ceil((end - start) * projected / 64.0), 1.0, 16.0));
    var visible = 0.0;
    for (var sample = 0u; sample < count; sample++) {
        let t = (f32(sample) + 0.5) / f32(count);
        visible += cloud_factor(eye + direction * mix(start, end, t));
    }
    return visible / f32(count);
}
