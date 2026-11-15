//! Ground fixtures are invented numerical cases, unrelated to any game's look.
use thrum_cloud_model::{
    Affinity, Fog, Ground, GroundProfile, GroundState, State, Surface, SurfacePoint,
};

/// Independent synthetic row with a physical extinction of 0.02 m⁻¹.
pub fn row() -> Fog {
    Fog {
        liquid_water: 0.000_133_333_333_333_333_34,
        droplet_radius: 10e-6,
        depth: 100.0,
        depression: 30.0,
        softness: 20.0,
        pooling: 0.0,
        drifting: false,
        affinity: Affinity::Any,
        centre: [0.0; 2],
        radius: [1000.0; 2],
        noise: 0.0,
        noise_scale: 40.0,
        radiative: false,
    }
}

/// Day constants taken as explicit test inputs, not an engine preset.
/// # Panics
/// Panics if the supplied rows are invalid.
pub fn table(rows: Vec<Fog>) -> Ground {
    Ground::new(
        GroundProfile {
            shore_margin: 40.0,
            calm: [2.0, 8.0],
            twilight: [-6.0, 10.0],
            cooling_seconds: 10_800.0,
            warming_seconds: 3600.0,
        },
        rows,
    )
    .expect("valid table")
}

/// A flat field covering the bank and Sun probes.
/// # Panics
/// Panics if the supplied height is nonfinite.
pub fn flat(height: f64) -> Surface {
    Surface::new(
        [-8000.0; 2],
        [2; 2],
        16_000.0,
        3000.0,
        &[SurfacePoint {
            ground: height,
            water: None,
        }; 4],
    )
    .expect("flat field")
}

/// Install test-authored amounts and moisture on the real weather state.
/// # Panics
/// Panics if a supplied control is outside its domain.
pub fn controls(state: &mut State, amounts: Vec<f64>, moisture: f64) {
    state.set_ground(GroundState::new(amounts, moisture, 60_000.0).expect("controls"));
}

/// Shore at x=0, with underwater relief and multiscale ridges on land.
/// Origin translation keeps all grid values identical for the frame identity.
/// # Panics
/// Panics if the supplied translation is nonfinite.
pub fn shore(shift: [f64; 2]) -> Surface {
    let mut points = Vec::new();
    for z in 0..9 {
        for x in 0..81 {
            let wx = f64::from(x - 40) * 20.0;
            let h = if wx <= 0.0 {
                -12.0 + 3.0 * (wx / 70.0).sin()
            } else {
                wx * 0.08 + 8.0 * (wx / 43.0).sin()
            };
            points.push(SurfacePoint {
                ground: h + f64::from(z) * 0.02,
                water: if wx <= 0.0 { Some(0.0) } else { None },
            });
        }
    }
    Surface::new(
        [-800.0 + shift[0], -80.0 + shift[1]],
        [81, 9],
        20.0,
        400.0,
        &points,
    )
    .expect("shore field")
}

/// Cubic ramp for independent expected values.
pub fn ramp(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
