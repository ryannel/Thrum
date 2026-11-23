//! Analytic shade stand-in for qualifying the atmosphere handoff, opt-in only.
use thrum_render::atmosphere::SunOpticalDepth;

pub(super) fn diagnostic() -> Result<SunOpticalDepth, thrum_render::error::RenderError> {
    const SIZE: u32 = 1024;
    const HALF_WIDTH: f32 = 32_768.0;
    let mut tau = Vec::with_capacity((SIZE * SIZE) as usize);
    for row in 0..SIZE {
        let z = (row as f32 + 0.5) / SIZE as f32 * 2.0 * HALF_WIDTH - HALF_WIDTH;
        for column in 0..SIZE {
            let x = (column as f32 + 0.5) / SIZE as f32 * 2.0 * HALF_WIDTH - HALF_WIDTH;
            let edge = ((HALF_WIDTH - x.abs().max(z.abs())) / 2048.0).clamp(0.0, 1.0);
            let pattern = (0.5 + 0.5 * (x / 128.0).sin() * (z / 192.0).cos()).powi(2);
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
}

/// Directional optical depth of a broken cloud deck, with two clearings.
/// The field supplies direct sunlight visibility without condensate.
pub(super) fn haze_banks() -> Result<SunOpticalDepth, String> {
    const EXTENT: u32 = 1024;
    const REGION: [f32; 4] = [-28_000.0, -10_500.0, 1_000.0, 5_500.0];
    let mut tau = Vec::with_capacity((EXTENT * EXTENT) as usize);
    let bank = |distance: f32| {
        let edge = ((1.0 - distance) / 0.30).clamp(0.0, 1.0);
        edge * edge * (3.0 - 2.0 * edge)
    };
    for row in 0..EXTENT {
        let z = REGION[1] + (row as f32 + 0.5) / EXTENT as f32 * (REGION[3] - REGION[1]);
        for column in 0..EXTENT {
            let x = REGION[0] + (column as f32 + 0.5) / EXTENT as f32 * (REGION[2] - REGION[0]);
            let deck = ((x + 15_900.0) / 12_000.0).powi(2) + ((z + 2_550.0) / 6_800.0).powi(2);
            // At a 10° Sun the first opening illuminates the plain east of
            // the lookout. The other opening falls farther up the valley.
            let near = ((x + 14_860.0) / 230.0).powi(2) + ((z + 2_480.0) / 140.0).powi(2);
            let far = ((x + 15_520.0) / 340.0).powi(2) + ((z + 2_485.0) / 120.0).powi(2);
            tau.push(4.6 * bank(deck) * (1.0 - bank(near).max(bank(far))));
        }
    }
    SunOpticalDepth::new([EXTENT; 2], &tau, 2_800.0, REGION, 0.001, 1)
        .map_err(|error| format!("diagnostic cloud banks: {error:?}"))
}
