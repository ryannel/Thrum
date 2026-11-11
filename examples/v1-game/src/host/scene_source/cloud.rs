//! Analytic shade stand-in for qualifying the atmosphere handoff, opt-in only.
use thrum_render::atmosphere::SunOpticalDepth;

pub(super) fn diagnostic() -> Result<SunOpticalDepth, String> {
    const SIZE: u32 = 256;
    const HALF_WIDTH: f32 = 32_768.0;
    let mut tau = Vec::with_capacity((SIZE * SIZE) as usize);
    for row in 0..SIZE {
        let z = (row as f32 + 0.5) / SIZE as f32 * 2.0 * HALF_WIDTH - HALF_WIDTH;
        for column in 0..SIZE {
            let x = (column as f32 + 0.5) / SIZE as f32 * 2.0 * HALF_WIDTH - HALF_WIDTH;
            let edge = ((HALF_WIDTH - x.abs().max(z.abs())) / 2048.0).clamp(0.0, 1.0);
            let pattern = (0.5 + 0.5 * (x / 1024.0).sin() * (z / 1536.0).cos()).powi(2);
            tau.push(2.0 * std::f32::consts::LN_2 * pattern * edge);
        }
    }
    SunOpticalDepth::new(
        [SIZE, SIZE],
        &tau,
        2000.0,
        [-HALF_WIDTH, -HALF_WIDTH, HALF_WIDTH, HALF_WIDTH],
        0.001,
        1,
    )
    .and_then(|field| field.with_drift([160.0, 40.0]))
    .map_err(|error| format!("diagnostic cloud shade: {error:?}"))
}
