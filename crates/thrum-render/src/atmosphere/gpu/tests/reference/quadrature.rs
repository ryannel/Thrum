//! Adaptive open Gauss-Legendre 8/4 pair. Shadow/ozone discontinuities are split
//! geometrically before integration; no endpoint convention biases a shadow edge.

use super::{Reason, Unresolved};

#[derive(Clone, Copy)]
pub(super) struct Settings {
    pub absolute: f64,
    pub relative: f64,
    pub depth: u32,
    pub evaluations: usize,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            absolute: 1e-9,
            relative: 1e-8,
            depth: 20,
            evaluations: 200_000,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Integral {
    pub value: f64,
    pub error: f64,
}

pub(super) fn integrate(
    points: &[f64],
    settings: Settings,
    mut function: impl FnMut(f64) -> Result<f64, Unresolved>,
) -> Result<Integral, Unresolved> {
    let mut remaining = settings.evaluations;
    let mut total = Integral {
        value: 0.0,
        error: 0.0,
    };
    let length = points.last().copied().unwrap_or(0.0) - points.first().copied().unwrap_or(0.0);
    if length <= 0.0 {
        return Ok(total);
    }
    for interval in points.windows(2) {
        let local = Settings {
            absolute: settings.absolute * (interval[1] - interval[0]) / length,
            ..settings
        };
        let result = adaptive(
            interval[0],
            interval[1],
            local,
            &mut remaining,
            &mut function,
        )?;
        total.value += result.value;
        total.error += result.error;
    }
    Ok(total)
}

fn adaptive(
    low: f64,
    high: f64,
    settings: Settings,
    remaining: &mut usize,
    function: &mut impl FnMut(f64) -> Result<f64, Unresolved>,
) -> Result<Integral, Unresolved> {
    if *remaining < 12 {
        return Err(Unresolved::new(Reason::QuadratureLimit));
    }
    *remaining -= 12;
    let centre = low.midpoint(high);
    let radius = (high - low) * 0.5;
    let mut rule = |nodes: &[(f64, f64)]| -> Result<f64, Unresolved> {
        let mut sum = 0.0;
        for &(node, weight) in nodes {
            sum += weight * (function(centre - radius * node)? + function(centre + radius * node)?);
        }
        Ok(sum * radius)
    };
    let fine = rule(&[
        (0.183_434_642_495_649_8, 0.362_683_783_378_362),
        (0.525_532_409_916_329, 0.313_706_645_877_887_3),
        (0.796_666_477_413_626_7, 0.222_381_034_453_374_5),
        (0.960_289_856_497_536_3, 0.101_228_536_290_376_3),
    ])?;
    let coarse = rule(&[
        (0.339_981_043_584_856_3, 0.652_145_154_862_546_1),
        (0.861_136_311_594_052_6, 0.347_854_845_137_453_8),
    ])?;
    if !fine.is_finite() || !coarse.is_finite() {
        return Err(Unresolved::new(Reason::NonFinite));
    }
    let error = (fine - coarse).abs();
    if error <= settings.absolute + settings.relative * fine.abs() {
        return Ok(Integral { value: fine, error });
    }
    if settings.depth == 0 {
        return Err(Unresolved::new(Reason::QuadratureLimit));
    }
    let next = Settings {
        absolute: settings.absolute * 0.5,
        depth: settings.depth - 1,
        ..settings
    };
    let left = adaptive(low, centre, next, remaining, function)?;
    let right = adaptive(centre, high, next, remaining, function)?;
    Ok(Integral {
        value: left.value + right.value,
        error: left.error + right.error,
    })
}
