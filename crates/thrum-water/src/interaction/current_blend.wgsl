struct CellCarriage { weights: array<vec4<f32>, CARRIAGE_COUNTS - 1u> }
@group(0) @binding(5) var<storage, read> carriage: array<CellCarriage>;

fn carriage_weight(index: u32) -> f32 {
    let count_minus_one = (step.level >> 8u) & 255u;
    if count_minus_one == 0u {
        return 1.0;
    }
    return carriage[step.cell_base + index].weights[count_minus_one - 1u][step.level >> 16u];
}
