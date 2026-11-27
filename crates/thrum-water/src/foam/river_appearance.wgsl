// Cross-filament holes use a world-metre kernel. Varying the current rotates
// only that kernel, so a turn cannot shear a whole world-space coordinate.
fn river_foam_cover(stocks: vec4<f32>, point: vec2<f32>, direction: vec2<f32>, footprint: f32) -> f32 {
    let cover = clamp(stocks.y + 0.16 * stocks.z, 0.0, 1.0);
    if cover < 0.0001 { return 0.0; }

    let grain = point * 9.2;
    let along = direction * 1.2;
    let lace = 0.28 * foam_noise(grain - along)
        + 0.44 * foam_noise(grain)
        + 0.28 * foam_noise(grain + along);
    let width = 0.055 + 0.14 * sqrt(cover);
    let threads = 1.0 - smoothstep(0.35 * width, width, abs(lace - 0.48));
    let openings = foam_noise(point * 3.5 + vec2(19.7, 5.3));
    let groups = smoothstep(0.76 - 0.65 * sqrt(cover), 0.91 - 0.48 * sqrt(cover), openings);
    let resolved = 0.83 * sqrt(cover) * threads * groups;
    let average = 0.28 * cover;
    let antialias = smoothstep(0.08, 0.30, footprint);
    return mix(resolved, average, antialias) * smoothstep(0.014, 0.063, cover);
}
