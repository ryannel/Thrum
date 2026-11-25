// Dispersion and shoaling depend on the dimensionless deep-water k * depth.
// Interpolate the immutable f64 solution in log space. Outside the table,
// use the same analytic evaluator as the drawn field.
@group(2) @binding(1) var<uniform> query_shoaling: array<vec4<f32>, QUERY_SHOALING_ROWS>;

fn shoaling_entry(index: u32) -> f32 {
    return query_shoaling[index / 4u][index % 4u];
}

fn wave_shoaling(k: f32, depth: f32) -> f32 {
    let position = (log2(k * depth) - QUERY_SHOALING_MIN) * QUERY_SHOALING_SCALE;
    if position < 0.0 || position >= f32(QUERY_SHOALING_ROWS * 4u - 1u) {
        return surf_shoaling(k, depth);
    }
    let index = u32(position);
    return mix(shoaling_entry(index), shoaling_entry(index + 1u), fract(position));
}
