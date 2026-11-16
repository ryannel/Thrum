@group(1) @binding(0) var<uniform> secondary: SecondaryViewRecord;

fn secondary_in_view(bounds: Bounds) -> bool {
    if secondary.options.x > 0.5 && outside(secondary.clip_plane, bounds.center, bounds.extent) {
        return false;
    }
    if secondary.options.z <= 0.0 { return true; }
    var minimum = vec2<f32>(1.0e30);
    var maximum = vec2<f32>(-1.0e30);
    for (var index = 0u; index < 8u; index++) {
        let sign = vec3(select(-1.0, 1.0, (index & 1u) != 0u), select(-1.0, 1.0, (index & 2u) != 0u), select(-1.0, 1.0, (index & 4u) != 0u));
        let clip = secondary.view_proj * vec4(bounds.center + sign * bounds.extent, 1.0);
        // A bound across the camera plane has unbounded projected size.
        if clip.w <= 0.0 { return true; }
        let screen = clip.xy / clip.w;
        minimum = min(minimum, screen);
        maximum = max(maximum, screen);
    }
    let pixels = (maximum - minimum) * 0.5 * secondary.extent.xy;
    return max(pixels.x, pixels.y) >= secondary.options.z;
}
