// Fresh river foam opens into elongated strands in the transported
// coordinates. The stocks still control birth, advection and fade.
fn river_foam_cover(stocks: vec4<f32>, point: vec2<f32>, direction: vec2<f32>, footprint: f32) -> f32 {
    let cover = clamp(stocks.y + stocks.z, 0.0, 1.0);
    if cover < 0.0001 { return 0.0; }
    let across = vec2(-direction.y, direction.x);
    let local = vec2(dot(point, direction), dot(point, across));
    let stretched = local * vec2(0.72, 4.8);
    let coarse = foam_noise(stretched);
    let fine = foam_noise(stretched * 3.2 + vec2(19.7, 5.3));
    let rank = 0.73 * coarse + 0.27 * fine;
    let stock = 0.09 + 0.49 * cover + 0.12 * stocks.y;
    let strands = 1.0 - smoothstep(stock - 0.09, stock + 0.09, rank);
    let resolved = sqrt(cover) * strands;
    let filtered = cover * (0.26 + 0.28 * stocks.y);
    return mix(resolved, filtered, smoothstep(0.10, 0.75, footprint))
        * smoothstep(0.012, 0.055, cover);
}
