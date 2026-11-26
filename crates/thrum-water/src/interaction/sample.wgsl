struct WakeSettings {
    origins: array<vec4<f32>, 4>,
    clocks: vec4<f32>,
    bases: vec4<u32>,
    fine_origins: array<vec4<f32>, 2>,
    fine_clock: vec4<f32>,
    fine_bases: vec4<u32>,
}

fn wake_cell(point: vec2<u32>, base: u32) -> f32 {
    return wake_endpoints[base + point.y * WAKE_SIDE + point.x].x;
}

fn wake_neighbour(point: vec2<i32>, base: u32) -> f32 {
    return wake_cell(vec2<u32>(clamp(point, vec2(0), vec2(i32(WAKE_SIDE) - 1))), base);
}

// Height and its spatial derivative, using the same interpolation for
// displacement, shading and game queries. The window does not repeat.
fn wake_plane(world: vec2<f32>, window: vec4<f32>, base: u32) -> vec3<f32> {
    let grid = (world - window.xy) / window.z;
    if any(grid < vec2(0.0)) || any(grid >= vec2(f32(WAKE_SIDE - 1u))) {
        return vec3(0.0);
    }
    let at = vec2<u32>(floor(grid));
    let w = fract(grid);
    let a = wake_cell(at, base);
    let b = wake_cell(at + vec2(1u, 0u), base);
    let c = wake_cell(at + vec2(0u, 1u), base);
    let d = wake_cell(at + vec2(1u, 1u), base);
    let height = mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
    if all(vec4(a,b,c,d) == vec4(0.0)) { return vec3(height, 0.0, 0.0); }
    // Interpolate the centred derivatives, so normals agree on both sides
    // of a sample-cell edge even at the narrow reflection of the Sun.
    let p = vec2<i32>(at);
    let slope_a = vec2(b - wake_neighbour(p - vec2(1, 0), base), c - wake_neighbour(p - vec2(0, 1), base));
    let slope_b = vec2(wake_neighbour(p + vec2(2, 0), base) - a, d - wake_neighbour(p + vec2(1, -1), base));
    let slope_c = vec2(d - wake_neighbour(p + vec2(-1, 1), base), wake_neighbour(p + vec2(0, 2), base) - a);
    let slope_d = vec2(wake_neighbour(p + vec2(2, 1), base) - c, wake_neighbour(p + vec2(1, 2), base) - b);
    let slope = mix(mix(slope_a, slope_b, w.x), mix(slope_c, slope_d, w.x), w.y);
    return vec3(height, slope / (2.0 * window.z));
}

fn wake_blend(previous: vec3<f32>, current: vec3<f32>, clock: vec4<f32>, offset: f32) -> vec4<f32> {
    let interval = max(1e-6, clock.y);
    let weight = clamp(clock.x + offset / interval, 0.0, 1.0);
    return vec4(mix(previous, current, weight), (current.x - previous.x) / interval);
}

fn wake_at(world: vec2<f32>, seconds_from_display: f32) -> vec4<f32> {
    if wake_settings.clocks.z == 0.0 {
        return vec4(0.0);
    }
    let previous = wake_plane(world, wake_settings.origins[0], wake_settings.bases.x)
        + wake_plane(world, wake_settings.origins[1], wake_settings.bases.y);
    let current = wake_plane(world, wake_settings.origins[2], wake_settings.bases.z)
        + wake_plane(world, wake_settings.origins[3], wake_settings.bases.w);
    var wake = wake_blend(previous, current, wake_settings.clocks, seconds_from_display);
    if wake_settings.fine_clock.z != 0.0 {
        let fine_previous = wake_plane(world, wake_settings.fine_origins[0], wake_settings.fine_bases.x);
        let fine_current = wake_plane(world, wake_settings.fine_origins[1], wake_settings.fine_bases.y);
        wake += wake_blend(fine_previous, fine_current, wake_settings.fine_clock, seconds_from_display);
    }
    return wake;
}
