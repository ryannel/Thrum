//! Continuous evaluation of the declared isotropic recurrence, not all-orders
//! transport truth. J1 is mean incident radiance / unit solar irradiance [sr^-1];
//! F is dimensionless. Only the single-scattering forcing changes between the
//! uniform-phase and true-phase seeds; both retain the same isotropic feedback.

use super::{
    Reason, Unresolved,
    geometry::{self, Vec3},
    quadrature, random,
    transport::{self, Medium, SingleSettings},
};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug)]
pub(super) struct Closure {
    pub j1_uniform: f64,
    pub j1_true_phase: f64,
    pub feedback: f64,
    pub m_uniform: f64,
    pub m_true_phase: f64,
}
impl Closure {
    fn values(self) -> [f64; 5] {
        [
            self.j1_uniform,
            self.j1_true_phase,
            self.feedback,
            self.m_uniform,
            self.m_true_phase,
        ]
    }
    fn from_terms([j1_uniform, j1_true_phase, feedback]: [f64; 3]) -> Result<Self, Unresolved> {
        // No production denominator floor: an inadmissible recurrence is
        // unresolved instead of being turned into a finite reference value.
        if [j1_uniform, j1_true_phase, feedback]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
            || feedback >= 1.0
        {
            return Err(Unresolved::new(Reason::NonFinite));
        }
        let result = Self {
            j1_uniform,
            j1_true_phase,
            feedback,
            m_uniform: j1_uniform / (1.0 - feedback),
            m_true_phase: j1_true_phase / (1.0 - feedback),
        };
        if result.values().iter().all(|v| v.is_finite()) {
            Ok(result)
        } else {
            Err(Unresolved::new(Reason::NonFinite))
        }
    }
}

// Generate Gauss-Legendre nodes by recurrence/Newton iteration, without the
// shader's Fibonacci directions, fixed march or stored quadrature constants.
fn legendre(n: u32, x: f64) -> (f64, f64) {
    let (mut previous, mut value) = (1.0, x);
    for k in 2..=n {
        let next = ((2.0 * f64::from(k) - 1.0) * x * value - (f64::from(k) - 1.0) * previous)
            / f64::from(k);
        previous = value;
        value = next;
    }
    (value, f64::from(n) * (x * value - previous) / (x * x - 1.0))
}
fn nodes(n: u32) -> Result<Vec<(f64, f64)>, Unresolved> {
    if !(2..=128).contains(&n) {
        return Err(Unresolved::new(Reason::InvalidCase));
    }
    (0..n)
        .map(|i| {
            let mut x = (PI * (f64::from(i) + 0.75) / (f64::from(n) + 0.5)).cos();
            for _ in 0..32 {
                let (p, derivative) = legendre(n, x);
                let step = p / derivative;
                x -= step;
                if step.abs() < 2e-15 {
                    let (_, derivative) = legendre(n, x);
                    return Ok((
                        x.midpoint(1.0),
                        1.0 / ((1.0 - x * x) * derivative * derivative),
                    ));
                }
            }
            Err(Unresolved::new(Reason::QuadratureLimit))
        })
        .collect()
}

fn sphere(height: f64, angular: u32) -> Result<Vec<(Vec3, f64)>, Unresolved> {
    let horizon =
        -(height * (2.0 * geometry::RADIUS + height)).sqrt() / (geometry::RADIUS + height);
    let nodes = nodes(angular)?;
    let mut rays = Vec::new();
    // Separate ground/space limits and concentrate around the curved horizon.
    // Weights integrate the SPHERE MEAN dOmega/(4pi), not projected irradiance.
    for (sign, span) in [(-1.0, 1.0 + horizon), (1.0, 1.0 - horizon)] {
        for &(t, weight) in &nodes {
            let mu = horizon + sign * span * t * t;
            let radial = (1.0 - mu * mu).max(0.0).sqrt();
            for j in 0..angular * 2 {
                let phi = 2.0 * PI * (f64::from(j) + 0.5) / f64::from(angular * 2);
                rays.push((
                    Vec3::new(radial * phi.cos(), mu, radial * phi.sin()),
                    span * t * weight / f64::from(angular * 2),
                ));
            }
        }
    }
    Ok(rays)
}

fn panels(origin: Vec3, direction: Vec3, length: f64, sun: Vec3, count: u32) -> Vec<f64> {
    let splits = geometry::splits(origin, direction, length, Some(sun));
    let mut result = Vec::new();
    for interval in splits.windows(2) {
        for i in 0..count {
            result
                .push(interval[0] + (interval[1] - interval[0]) * f64::from(i) / f64::from(count));
        }
    }
    result.push(length);
    result
}

fn ray(
    origin: Vec3,
    sun: Vec3,
    direction: Vec3,
    medium: Medium,
    path: SingleSettings,
) -> Result<[f64; 3], Unresolved> {
    let boundary = geometry::boundary(origin, direction);
    let points = panels(
        origin,
        direction,
        boundary.distance,
        sun,
        path.radiance_subdivisions,
    );
    let cosine = direction.dot(sun).clamp(-1.0, 1.0);
    let mut terms = [0.0; 3];
    for (term, result) in terms.iter_mut().enumerate() {
        *result = quadrature::integrate(&points, path.radiance, |distance| {
            let point = origin + direction * distance;
            let c = medium.at(point);
            let view = (-transport::optical_depth(
                origin,
                direction,
                distance,
                medium,
                path.optical_depth,
            )?)
            .exp();
            if term == 2 {
                return Ok(view * (c.rayleigh + c.mie));
            }
            let sun_t = transport::exact_sun(point, sun, medium, path.optical_depth)?;
            let forcing = if term == 0 {
                (c.rayleigh + c.mie) / (4.0 * PI)
            } else {
                c.rayleigh * random::rayleigh(cosine) + c.mie * random::hg(cosine)
            };
            Ok(view * sun_t * forcing)
        })?
        .value;
    }
    if boundary.ground {
        let point = (origin + direction * boundary.distance).unit() * geometry::RADIUS;
        let view = (-transport::optical_depth(
            origin,
            direction,
            boundary.distance,
            medium,
            path.optical_depth,
        )?)
        .exp();
        let ground = view * medium.ground_albedo / PI
            * point.unit().dot(sun).max(0.0)
            * transport::exact_sun(point, sun, medium, path.optical_depth)?;
        terms[0] += ground;
        terms[1] += ground;
        terms[2] += view * medium.ground_albedo;
    }
    Ok(terms)
}

/// Evaluate both forcing choices at physical coordinates with the same medium,
/// direct Lambert ground and isotropic recurrence. No LUT sampling or GPU use.
pub(super) fn evaluate(
    height: f64,
    sun_mu: f64,
    medium: Medium,
    angular: u32,
    path: SingleSettings,
) -> Result<Closure, Unresolved> {
    if !height.is_finite()
        || !(0.0..=geometry::HEIGHT).contains(&height)
        || !sun_mu.is_finite()
        || !(-1.0..=1.0).contains(&sun_mu)
        || !(1..=64).contains(&path.radiance_subdivisions)
        || [
            medium.rayleigh,
            medium.mie_scattering,
            medium.mie_extinction,
            medium.ozone,
            medium.ground_albedo,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0)
        || medium.ground_albedo > 1.0
        || medium.mie_scattering > medium.mie_extinction
    {
        return Err(Unresolved::new(Reason::InvalidCase));
    }
    let origin = Vec3::new(0.0, geometry::RADIUS + height, 0.0);
    let sun = Vec3::new((1.0 - sun_mu * sun_mu).max(0.0).sqrt(), sun_mu, 0.0);
    let mut terms = [0.0; 3];
    for (direction, weight) in sphere(height, angular)? {
        for (sum, value) in terms
            .iter_mut()
            .zip(ray(origin, sun, direction, medium, path)?)
        {
            *sum += weight * value;
        }
    }
    Closure::from_terms(terms)
}

fn vacuum(albedo: f64) -> Medium {
    Medium {
        rayleigh: 0.0,
        mie_scattering: 0.0,
        mie_extinction: 0.0,
        ozone: 0.0,
        ground_albedo: albedo,
    }
}

#[test]
fn sphere_mean_phase_and_zero_scattering_have_analytic_normalizations() {
    for sun_mu in [-0.05, 0.0, 1.0] {
        let sun = Vec3::new((1.0_f64 - sun_mu * sun_mu).sqrt(), sun_mu, 0.0);
        let mut sums = [0.0; 5];
        for (direction, weight) in sphere(2.0, 64).expect("angular cubature") {
            let mu = direction.dot(sun);
            for (sum, value) in sums.iter_mut().zip([
                1.0,
                4.0 * PI * random::rayleigh(mu),
                4.0 * PI * random::hg(mu),
                4.0 * PI * random::rayleigh(mu) * mu,
                4.0 * PI * random::hg(mu) * mu,
            ]) {
                *sum += weight * value;
            }
        }
        for (actual, expected) in sums.into_iter().zip([1.0, 1.0, 1.0, 0.0, 0.8]) {
            assert!(
                (actual - expected).abs() < 1e-6,
                "sphere/phase mean {actual}, expected {expected}"
            );
        }
    }
    let path = SingleSettings::default();
    let black = evaluate(2.0, 0.0, vacuum(0.0), 2, path).expect("vacuum black ground");
    assert!(black.values().iter().all(|v| v.abs() < 1e-15));
    let absorbing = Medium {
        mie_extinction: 4.4e-6,
        ..vacuum(0.0)
    };
    assert!(
        evaluate(2.0, 0.0, absorbing, 2, path)
            .expect("zero scattering")
            .values()
            .iter()
            .all(|v| v.abs() < 1e-15)
    );
    assert!(
        Closure::from_terms([1.0, 1.0, 1.0]).is_err(),
        "singular feedback must not be floored"
    );
}

#[test]
fn vacuum_ground_local_closure_self_feedback_is_not_transport_truth() {
    let path = SingleSettings::default();
    let rho = 0.25;
    let ground = evaluate(0.0, 1.0, vacuum(rho), 2, path).expect("vacuum Lambert ground");
    let physical_mean = rho / (2.0 * PI);
    // At the surface with a zenith Sun, half the directions see Lambert
    // radiance rho/pi and half see empty space. In vacuum every reflected
    // outward ray escapes the convex planet: physical indirect returns are zero.
    let surface = Vec3::new(0.0, geometry::RADIUS, 0.0);
    for mu in [0.1, 0.5, 1.0] {
        let outward = Vec3::new((1.0_f64 - mu * mu).sqrt(), mu, 0.0);
        assert!(!geometry::boundary(surface, outward).ground);
    }
    assert!((ground.feedback - rho / 2.0).abs() < 1e-12);
    assert!((ground.j1_uniform - physical_mean).abs() < 1e-12);
    assert!((ground.j1_true_phase - ground.j1_uniform).abs() < 1e-12);
    // The inherited local isotropic recurrence nevertheless feeds a fraction
    // rho/2 back into this same cell. Its geometric amplification is a closure
    // approximation, not the physically expected vacuum all-orders solution.
    assert!((ground.m_uniform - physical_mean / (1.0 - rho / 2.0)).abs() < 1e-12);
    assert!(ground.m_uniform > physical_mean);
    assert!((ground.m_true_phase - ground.m_uniform).abs() < 1e-12);
}

#[test]
fn conservative_feedback_matches_escape_probability() {
    let medium = Medium {
        rayleigh: 5.802e-6,
        ..vacuum(1.0)
    };
    let path = SingleSettings::default();
    let origin = Vec3::new(0.0, geometry::RADIUS + 2.0, 0.0);
    let mut escape = 0.0;
    for (direction, weight) in sphere(2.0, 2).expect("small energy cubature") {
        let boundary = geometry::boundary(origin, direction);
        if !boundary.ground {
            escape += weight
                * (-transport::optical_depth(
                    origin,
                    direction,
                    boundary.distance,
                    medium,
                    path.optical_depth,
                )
                .expect("escape depth"))
                .exp();
        }
    }
    let value = evaluate(2.0, 1.0, medium, 2, path).expect("conservative medium with escape");
    assert!(value.feedback > 0.0 && value.feedback < 1.0);
    assert!(
        (value.feedback + escape - 1.0).abs() < 1e-7,
        "feedback + escape = {}",
        value.feedback + escape
    );
}

#[test]
fn continuous_multiple_reports_requested_refinements() {
    let Ok(requested) = std::env::var("THRUM_G3S_MULTIPLE_REFERENCE") else {
        eprintln!(
            "continuous M physical qualification not_requested; analytic falsifiers run separately"
        );
        return;
    };
    let angular = requested
        .parse::<u32>()
        .expect("integer base angular order");
    assert!(
        (2..=32).contains(&angular),
        "bounded base angular order 2..32"
    );
    let selected_case = std::env::var("THRUM_G3S_MULTIPLE_CASE").map_or(0, |value| {
        value.parse::<usize>().expect("case 0=all or 1..4")
    });
    assert!(selected_case <= 4, "case 0=all or 1..4");
    let axis = std::env::var("THRUM_G3S_MULTIPLE_AXIS").unwrap_or_else(|_| "both".to_owned());
    assert!(
        ["both", "angular", "path"].contains(&axis.as_str()),
        "axis both/angular/path"
    );
    let angular_requested = axis != "path";
    let path_requested = axis != "angular";
    let path = SingleSettings::default();
    let tighter = SingleSettings {
        optical_depth: quadrature::Settings {
            absolute: 1e-11,
            relative: 1e-10,
            ..path.optical_depth
        },
        radiance: quadrature::Settings {
            absolute: 1e-16,
            relative: 1e-10,
            ..path.radiance
        },
        radiance_subdivisions: 4,
    };
    eprintln!(
        "# Isotropic closure referee only; no LUT/MC agreement or physical admission asserted. J1,M [sr^-1], F dimensionless; empirical refinement is not a certified error bound."
    );
    eprintln!(
        "# request case={selected_case} axis={axis} base_angular={angular}; unrequested axes are not evaluated or claimed resolved"
    );
    eprintln!(
        "band\tsun_deg\theight_m\taxis\tangular\tJ1_uniform\tJ1_true_phase\tF\tM_uniform\tM_true_phase\tmax_scaled_delta\tstatus\tcase"
    );
    let mut resolved = true;
    // Two held-out near-surface cells, then two EXACT old-grid node positions.
    // The latter remove M interpolation from a subsequent GPU comparison;
    // transmittance-LUT error remains separate from this exact-depth referee.
    for (index, (band, height, sun_mu)) in [
        (2, 2.0, 0.0),
        (1, 2.0, (-3.0_f64).to_radians().sin()),
        (2, 1562.5, -1.0 / 32.0),
        (1, 1562.5, -3.0 / 32.0),
    ]
    .into_iter()
    .enumerate()
    {
        let case = index + 1;
        if selected_case != 0 && selected_case != case {
            continue;
        }
        let sun_deg = sun_mu.asin().to_degrees();
        let coarse = evaluate(height, sun_mu, Medium::band(band), angular, path);
        let fine_angular = if angular_requested {
            angular * 2
        } else {
            angular
        };
        let fine = if angular_requested {
            evaluate(height, sun_mu, Medium::band(band), fine_angular, path)
        } else {
            coarse
        };
        let mut rows = vec![("base", angular, coarse, None)];
        if angular_requested {
            rows.push(("angular", fine_angular, fine, Some(coarse)));
        }
        if path_requested {
            rows.push((
                "path",
                fine_angular,
                evaluate(height, sun_mu, Medium::band(band), fine_angular, tighter),
                Some(fine),
            ));
        }
        for (axis, n, result, previous) in rows {
            if let Ok(value) = result {
                let comparison_failed = previous.is_some_and(|old| old.is_err());
                let delta = previous.and_then(Result::ok).map(|old| {
                    old.values()
                        .into_iter()
                        .zip(value.values())
                        .map(|(a, b)| (a - b).abs() / (1e-12 + 0.0025 * b.abs()))
                        .fold(0.0_f64, f64::max)
                });
                let status = if comparison_failed {
                    resolved = false;
                    "unresolved_previous"
                } else if delta.is_some_and(|v| v > 1.0) {
                    resolved = false;
                    "unresolved"
                } else if delta.is_some() {
                    "refinement_agreement"
                } else {
                    "unrefined"
                };
                eprintln!(
                    "{band}\t{sun_deg}\t{height}\t{axis}\t{n}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{}\t{status}\t{case}",
                    value.j1_uniform,
                    value.j1_true_phase,
                    value.feedback,
                    value.m_uniform,
                    value.m_true_phase,
                    delta.map_or_else(|| "NA".to_owned(), |v| format!("{v:.17e}"))
                );
            } else {
                resolved = false;
                eprintln!(
                    "{band}\t{sun_deg}\t{height}\t{axis}\t{n}\tNA\tNA\tNA\tNA\tNA\tNA\tunresolved_transport\t{case}"
                );
                eprintln!("# case={case} axis={axis} failure={result:?}");
            }
        }
    }
    assert!(
        resolved,
        "requested continuous M numerical refinement unresolved; inspect every reported row"
    );
}
