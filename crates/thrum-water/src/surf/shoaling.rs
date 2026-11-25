//! A small immutable dimensionless shoaling table for the shared surf evaluator.
pub(super) const ROWS: usize = 256;
const MIN_LOG_DEPTH: f64 = -20.0;
const MAX_LOG_DEPTH: f64 = 5.0;
const SCALE: f64 = (4 * ROWS - 1) as f64 / (MAX_LOG_DEPTH - MIN_LOG_DEPTH);

pub(super) fn source() -> String {
    format!(
        "const SHOALING_ROWS = {ROWS}u;\nconst SHOALING_MIN = {MIN_LOG_DEPTH:.1};\n        const SHOALING_SCALE = {SCALE:.9};\n"
    )
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the query evaluates its shader in f32"
)]
pub(super) fn table() -> [[f32; 4]; ROWS] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let i = row * 4 + column;
            let depth = (MIN_LOG_DEPTH + i as f64 / SCALE).exp2();
            thrum_water_model::shoaling(1.0, depth) as f32
        })
    })
}
