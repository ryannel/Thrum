struct WakeSettings {
    origins: array<vec4<f32>, 4>,
    clocks: vec4<f32>,
    bases: vec4<u32>,
}

fn wake_cell(point: vec2<u32>, plane: u32) -> f32 {
    return wake_endpoints[wake_settings.bases[plane] + point.y * WAKE_SIDE + point.x].x;
}

fn wake_neighbour(point: vec2<i32>, plane: u32) -> f32 {
    return wake_cell(vec2<u32>(clamp(point, vec2(0), vec2(i32(WAKE_SIDE) - 1))), plane);
}

// Height and its spatial derivative, using the same interpolation for
// displacement, shading and game queries. The window does not repeat.
fn wake_plane(world: vec2<f32>, plane: u32) -> vec3<f32> {
    let window = wake_settings.origins[plane];
    let grid = (world - window.xy) / window.z;
    if any(grid < vec2(0.0)) || any(grid >= vec2(f32(WAKE_SIDE - 1u))) {
        return vec3(0.0);
    }
    let at = vec2<u32>(floor(grid));
    let w = fract(grid);
    let a = wake_cell(at, plane);
    let b = wake_cell(at + vec2(1u, 0u), plane);
    let c = wake_cell(at + vec2(0u, 1u), plane);
    let d = wake_cell(at + vec2(1u, 1u), plane);
    let height = mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
    if all(vec4(a,b,c,d) == vec4(0.0)) { return vec3(height, 0.0, 0.0); }
    // Interpolate the centred derivatives, so normals agree on both sides
    // of a sample-cell edge even at the narrow reflection of the Sun.
    let p = vec2<i32>(at);
    let slope_a = vec2(b - wake_neighbour(p - vec2(1, 0), plane), c - wake_neighbour(p - vec2(0, 1), plane));
    let slope_b = vec2(wake_neighbour(p + vec2(2, 0), plane) - a, d - wake_neighbour(p + vec2(1, -1), plane));
    let slope_c = vec2(d - wake_neighbour(p + vec2(-1, 1), plane), wake_neighbour(p + vec2(0, 2), plane) - a);
    let slope_d = vec2(wake_neighbour(p + vec2(2, 1), plane) - c, wake_neighbour(p + vec2(1, 2), plane) - b);
    let slope = mix(mix(slope_a, slope_b, w.x), mix(slope_c, slope_d, w.x), w.y);
    return vec3(height, slope / (2.0 * window.z));
}

fn wake_at(world: vec2<f32>, seconds_from_display: f32) -> vec4<f32> {
    if wake_settings.clocks.z == 0.0 {
        return vec4(0.0);
    }
    let previous = wake_plane(world, 0u) + wake_plane(world, 1u);
    let current = wake_plane(world, 2u) + wake_plane(world, 3u);
    let interval = wake_settings.clocks.y;
    let weight = clamp(wake_settings.clocks.x + seconds_from_display / max(1e-6, interval), 0.0, 1.0);
    let rate = (current.x - previous.x) / max(1e-6, interval);
    return vec4(mix(previous, current, weight), rate);
}
