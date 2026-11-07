// One profile, two independent charts. Red stores terminal night radiance
// and green stores the local first-scatter source per metre.
@group(0) @binding(6) var night_profile: texture_2d<f32>;
@group(0) @binding(7) var<uniform> night_control: vec4<u32>;

fn night_terminal(h: f32, mu: f32, tangent: f32, ground: bool, sampler_linear: sampler) -> f32 {
    if night_control.x == 0u { return 0.0; }
    let size = vec2<f32>(textureDimensions(night_profile));
    let side = 0.5 * size.x;
    let fraction = select((mu - tangent) / (1.0 - tangent),
                          (tangent - mu) / (1.0 + tangent), ground);
    let column = select(side, 0.0, ground) + sqrt(clamp(fraction, 0.0, 1.0)) * (side - 1.0);
    let row = sqrt(clamp(h / 15000.0, 0.0, 1.0)) * (size.y - 1.0);
    let uv = (vec2(column, row) + 0.5) / size;
    return textureSampleLevel(night_profile, sampler_linear, uv, 0.0).r;
}

fn night_source(h: f32, mu: f32, sampler_linear: sampler) -> f32 {
    let size = vec2<f32>(textureDimensions(night_profile));
    let x = 0.5 + 0.5 * sign(mu) * sqrt(clamp(abs(mu), 0.0, 1.0));
    let y = sqrt(clamp(h / 95000.0, 0.0, 1.0));
    let uv = (vec2(x, y) * (size - 1.0) + 0.5) / size;
    // Convert the source's 2^30 storage gain to the radiance channel's 2^16
    // gain once. The marcher integrates in that scale through to its write.
    return textureSampleLevel(night_profile, sampler_linear, uv, 0.0).g / 16384.0;
}
