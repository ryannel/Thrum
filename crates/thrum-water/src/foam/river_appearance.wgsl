// Cross-filament holes use a world-metre kernel. Varying the current rotates
// only that kernel, so a turn cannot shear a whole world-space coordinate.
fn river_foam_grain(point: vec2<f32>, direction: vec2<f32>) -> vec2<f32> {
    let grain = point * 9.2;
    let along = direction * 1.2;
    let lace = 0.28 * foam_noise(grain - along)
        + 0.44 * foam_noise(grain)
        + 0.28 * foam_noise(grain + along);
    return vec2(lace, foam_noise(point * 3.5 + vec2(19.7, 5.3)));
}

fn river_foam_cover(stocks: vec4<f32>, point: vec2<f32>, current: vec2<f32>, track: vec4<f32>, footprint: f32) -> f32 {
    let cover = clamp(stocks.y + 0.16 * stocks.z, 0.0, 1.0);
    if cover < 0.0001 { return 0.0; }

    // Phase is continuous across the chart but varies from one part of a
    // trail to another. A reading changes its seed only at zero weight.
    var span = water.carriage.y;
    var clock = vec2(water.carriage.x, water.carriage.z);
    if track.w < 0.0 {
        // Keep the distance a lace reading covers in a sheared current
        // within the chart's bound on spatial deformation.
        span = min(span, track.z);
        let cycles = water.carriage.w / span;
        clock = vec2(fract(cycles), floor(cycles) % 256.0);
    }
    let phase = clock.x + 2.0 * foam_noise(point * 0.23 + vec2(27.0, 45.0));
    let reading = carried_points(point, current, vec2(phase, span));
    let cycle = u32(clock.y + floor(phase));
    let other = cycle + u32(fract(phase) >= 0.5);
    let direction = current / max(0.05, length(current));
    let a = river_foam_grain(reading.first + foam_renewal(cycle, 11.0), direction);
    let b = river_foam_grain(reading.second + foam_renewal(other, 32.0), direction);
    // Renew the noise before taking the thread and opening contours.
    // Blending finished strands instead makes both sets turn translucent.
    let weights = vec2(reading.weight, 1.0 - reading.weight);
    let noise = 0.5 + (mix(b, a, reading.weight) - 0.5) * inverseSqrt(dot(weights, weights));
    let width = 0.055 + 0.14 * sqrt(cover);
    let threads = 1.0 - smoothstep(0.35 * width, width, abs(noise.x - 0.48));
    let groups = smoothstep(0.76 - 0.65 * sqrt(cover), 0.91 - 0.48 * sqrt(cover), noise.y);
    let resolved = 0.76 * sqrt(cover) * threads * groups;
    let average = 0.28 * cover;
    let antialias = smoothstep(0.08, 0.30, footprint);
    return mix(resolved, average, antialias) * smoothstep(0.014, 0.063, cover);
}
