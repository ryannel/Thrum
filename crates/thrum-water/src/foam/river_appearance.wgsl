// Make elongated openings by sampling along the flow in world metres. A
// varying direction rotates only the kernel, so it cannot stretch a whole
// world-space coordinate when the current turns.
fn river_foam_cover(stocks: vec4<f32>, point: vec2<f32>, direction: vec2<f32>, footprint: f32) -> f32 {
    let stock = stocks.y + 0.32 * stocks.z;
    let cover = clamp(stock, 0.0, 1.0);
    if cover < 0.0001 { return 0.0; }
    let at = point * 4.4;
    let axis = direction * 0.81;
    let strand = 0.28 * foam_noise(at - axis) + 0.44 * foam_noise(at) + 0.28 * foam_noise(at + axis);
    let opening = foam_noise(at * 0.57 + vec2(19.7, 5.3));
    let grain = foam_noise(at * 2.2 + vec2(-9.3, 8.1));
    let rank = 0.62 * strand + 0.24 * opening + 0.14 * grain;
    let level = 0.12 + 0.49 * cover + 0.13 * stocks.y;
    let strands = 1.0 - smoothstep(level - 0.08, level + 0.08, rank);
    let resolved = sqrt(cover) * strands;
    let filtered = cover * (0.26 + 0.28 * stocks.y);
    return mix(resolved, filtered, smoothstep(0.11, 0.65, footprint))
        * smoothstep(0.014, 0.072, cover);
}
