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

/// Two soft-edged banks with an open gap over the valley. The field is a
/// stand-in for directional optical depth, with no condensate of its own.
pub(super) fn haze_banks() -> Result<SunOpticalDepth, String> {
    const EXTENT: u32 = 256;
    const REGION: [f32; 4] = [-13_500.0, -4_500.0, 2_000.0, 2_000.0];
    let mut tau = Vec::with_capacity((EXTENT * EXTENT) as usize);
    let bank = |distance: f32| {
        let edge = ((1.0 - distance) / 0.30).clamp(0.0, 1.0);
        edge * edge * (3.0 - 2.0 * edge)
    };
    for row in 0..EXTENT {
        let z = REGION[1] + (row as f32 + 0.5) / EXTENT as f32 * (REGION[3] - REGION[1]);
        for column in 0..EXTENT {
            let x = REGION[0] + (column as f32 + 0.5) / EXTENT as f32 * (REGION[2] - REGION[0]);
            let along = x + 7_060.0;
            // The low Sun's horizontal bearing runs along the gap. Its
            // incoming rays stay in the gap from the ridge to the valley.
            let across = z + 1_105.0 - 0.179 * along;
            let west = ((along + 500.0) / 4_300.0).powi(2) + ((across + 400.0) / 300.0).powi(2);
            let east = ((along - 550.0) / 3_950.0).powi(2) + ((across - 380.0) / 270.0).powi(2);
            tau.push(3.8 * bank(west).max(bank(east)));
        }
    }
    SunOpticalDepth::new([EXTENT; 2], &tau, 1_450.0, REGION, 0.001, 1)
        .map_err(|error| format!("diagnostic cloud banks: {error:?}"))
}
