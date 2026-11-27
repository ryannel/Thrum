fn caustic_column(point: vec2<f32>, window: vec4<f32>, columns: texture_2d<f32>) -> vec4<f32> {
    let uv = (point - window.xy) / window.z;
    if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) { return vec4(0.0); }
    let size = vec2<f32>(textureDimensions(columns));
    return textureLoad(columns, vec2<i32>(uv * size), 0);
}

// The ladder is addressed at the surface exit of the mean refracted ray.
// A sample outside the window has unit irradiance.
fn caustic_field_at(
    exit: vec2<f32>, depth: f32, footprint: f32, window: vec4<f32>,
    field: texture_2d_array<f32>, filtered: sampler, active_rungs: u32,
) -> f32 {
    let uv = (exit - window.xy) / window.z;
    let edge = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let border = smoothstep(0.0, 0.125, edge);
    if border == 0.0 || depth <= 0.0 { return 1.0; }
    let last = f32(min(textureNumLayers(field), max(active_rungs, 1u)) - 1u);
    let rung = clamp(2.0 * log2(depth / 0.25), 0.0, last);
    let first = floor(rung);
    let cell = window.z / f32(textureDimensions(field).x);
    let mip = log2(max(1.0, footprint / cell));
    let a = textureSampleLevel(field, filtered, uv, i32(first), mip).r;
    let b = textureSampleLevel(field, filtered, uv, i32(ceil(rung)), mip).r;
    let shallow = min(depth / 0.25, 1.0);
    return 1.0 + border * shallow * (mix(a, b, fract(rung)) - 1.0);
}
