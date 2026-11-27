// Sample the whitewater noise on a closed departure-time domain. Each
// octave has whole cells on the cascades' loop, so its value and slope meet.
fn sheet_noise(point: vec2<f32>, span: f32, frequency: vec2<f32>, offset: vec2<f32>) -> f32 {
    let period = max(1.0, floor(span * frequency.y + 0.5));
    let p = vec2(point.x * frequency.x, point.y * period / max(span, 0.001)) + offset;
    let tile = floor(p);
    let f = fract(p);
    let w = f * f * (3.0 - 2.0 * f);
    let a = tile.y - floor(tile.y / period) * period;
    let b = a + 1.0 - floor((a + 1.0) / period) * period;
    return mix(
        mix(noise_hash(vec2(tile.x, a)), noise_hash(vec2(tile.x + 1.0, a)), w.x),
        mix(noise_hash(vec2(tile.x, b)), noise_hash(vec2(tile.x + 1.0, b)), w.x),
        w.y);
}
