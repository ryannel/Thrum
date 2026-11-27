fn noise_hash(at: vec2<f32>) -> f32 {
    var p = fract(vec3(at.x, at.y, at.x) * vec3(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}
fn foam_noise(p: vec2<f32>) -> f32 {
    let tile = floor(p);
    let f = fract(p);
    let w = f*f*(3.0 - 2.0*f);
    return mix(mix(noise_hash(tile), noise_hash(tile + vec2(1.0, 0.0)), w.x),
        mix(noise_hash(tile + vec2(0.0, 1.0)), noise_hash(tile + 1.0), w.x), w.y);
}
