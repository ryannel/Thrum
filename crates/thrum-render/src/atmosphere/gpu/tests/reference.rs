//! Matched-medium numerical reference, not measured spectral/CIE/lux truth.
//! Only the declared coefficient data matches common.wgsl. Geometry, adaptive
//! quadrature and scalar all-orders transport are independent of its LUT closure.
//! Methods: pbr-book.org/4ed/Light_Transport_II_Volume_Rendering/Volume_Scattering_Integrators
//! and cs.dartmouth.edu/~wjarosz/publications/novak14residual.html. No source copied.

mod geometry;
mod gpu;
mod quadrature;
mod random;
mod transport;

use geometry::Vec3;
use transport::{Medium, PathValue};

const BATCHES: usize = 8;
const EVENT_LIMIT: usize = 100_000;

/// Elevation/azimuth in radians; altitude in metres. Bands are coefficient
/// channels 0..3, with no claim that these are measured monochromatic spectra.
#[derive(Clone, Copy, Debug)]
pub(super) struct Case {
    pub band: usize,
    pub sun_elevation: f64,
    pub view_elevation: f64,
    pub relative_azimuth: f64,
    pub height: f64,
}

impl Case {
    pub(super) fn degrees(band: usize, sun: f64, view: f64, azimuth: f64, height: f64) -> Self {
        Self {
            band,
            sun_elevation: sun.to_radians(),
            view_elevation: view.to_radians(),
            relative_azimuth: azimuth.to_radians(),
            height,
        }
    }

    fn vectors(self) -> Result<(Vec3, Vec3, Vec3), Unresolved> {
        if self.band >= 3
            || ![
                self.sun_elevation,
                self.view_elevation,
                self.relative_azimuth,
                self.height,
            ]
            .into_iter()
            .all(f64::is_finite)
            || self.sun_elevation.abs() > std::f64::consts::FRAC_PI_2
            || self.view_elevation.abs() > std::f64::consts::FRAC_PI_2
            || !(0.0..=geometry::HEIGHT).contains(&self.height)
        {
            return Err(Unresolved::new(Reason::InvalidCase));
        }
        let sun = Vec3::new(self.sun_elevation.cos(), self.sun_elevation.sin(), 0.0);
        let view = Vec3::new(
            self.view_elevation.cos() * self.relative_azimuth.cos(),
            self.view_elevation.sin(),
            self.view_elevation.cos() * self.relative_azimuth.sin(),
        );
        Ok((
            Vec3::new(0.0, geometry::RADIUS + self.height, 0.0),
            sun,
            view,
        ))
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Reason {
    InvalidCase,
    InvalidPathCount,
    EventLimit,
    QuadratureLimit,
    NonFinite,
    MajorantViolation,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Unresolved {
    pub reason: Reason,
    pub completed_paths: usize,
    pub requested_paths: usize,
}
impl Unresolved {
    fn new(reason: Reason) -> Self {
        Self {
            reason,
            completed_paths: 0,
            requested_paths: 0,
        }
    }
}

/// Empirical batch uncertainty, not a confidence certificate or a rare-event bound.
#[derive(Clone, Copy, Debug)]
pub(super) struct Estimate {
    pub mean: f64,
    pub standard_error: f64,
}

#[derive(Clone, Debug)]
pub(super) struct Stats {
    pub radiance: Estimate,
    /// Atmospheric collision count, bins 1,2,3,4,5+ (no truncation).
    /// Includes coupled ground paths and is not the deterministic single term.
    pub by_order: [Estimate; 5],
    /// One atmospheric event and no ground events before the Sun connection.
    pub direct_volume: Estimate,
    /// Zero atmospheric events, first ground hit (the explicit order-zero term).
    pub direct_ground: Estimate,
    /// Contributions whose next-event connection vertex is on the ground.
    pub ground_vertices: Estimate,
    /// Contributions from paths that have never touched the ground.
    pub atmosphere: Estimate,
    /// Contributions at/after any ground interaction, including indirect returns.
    pub ground: Estimate,
    pub batch_means: [f64; BATCHES],
    pub zero_batches: usize,
    pub nonzero_paths: usize,
    pub nonzero_by_order: [usize; 6],
    pub largest_path: f64,
    pub largest_event: f64,
    pub max_interactions: usize,
    pub paths: usize,
    pub seed: u64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Single {
    pub volume: f64,
    pub unscattered_ground: f64,
    pub radiance: f64,
    /// Adaptive outer-integral estimate; not a bound on all nested quadrature error.
    pub estimated_error: f64,
}

struct Budget {
    remaining: usize,
}
impl Budget {
    fn spend(&mut self) -> Result<(), Unresolved> {
        if self.remaining == 0 {
            return Err(Unresolved::new(Reason::EventLimit));
        }
        self.remaining -= 1;
        Ok(())
    }
}

pub(super) fn deterministic_single(case: Case) -> Result<Single, Unresolved> {
    let (camera, sun, view) = case.vectors()?;
    transport::single(
        camera,
        sun,
        view,
        Medium::band(case.band),
        transport::SingleSettings::default(),
    )
}

/// Eight equally sized independent PRNG batch streams. A zero batch/sample count
/// is exposed explicitly: mean=SE=0 from rare-path sampling does not prove darkness.
pub(super) fn estimate(case: Case, paths: usize, seed: u64) -> Result<Stats, Unresolved> {
    case.vectors()?;
    estimate_medium(case, Medium::band(case.band), paths, seed, EVENT_LIMIT)
}

fn estimate_medium(
    case: Case,
    medium: Medium,
    paths: usize,
    seed: u64,
    limit: usize,
) -> Result<Stats, Unresolved> {
    let (camera, sun, view) = case.vectors()?;
    if paths == 0 || !paths.is_multiple_of(BATCHES) {
        return Err(Unresolved {
            requested_paths: paths,
            ..Unresolved::new(Reason::InvalidPathCount)
        });
    }
    let mut sums = [[0.0; 10]; BATCHES];
    let mut nonzero_paths = 0;
    let mut nonzero_by_order = [0; 6];
    let mut largest_path = 0.0_f64;
    let mut largest_event = 0.0_f64;
    let mut max_interactions = 0;
    for index in 0..paths {
        let mut streams = random::Streams::new(seed, index % BATCHES, index / BATCHES);
        let value = transport::trace(
            camera,
            sun,
            view,
            medium,
            &mut streams,
            &mut Budget { remaining: limit },
            transport::TraceOptions::default(),
        )
        .map_err(|error| Unresolved {
            completed_paths: index,
            requested_paths: paths,
            ..error
        })?;
        let total = value.orders.iter().sum::<f64>();
        nonzero_paths += usize::from(total > 0.0);
        for (order, &contribution) in value.orders.iter().enumerate() {
            nonzero_by_order[order] += usize::from(contribution > 0.0);
        }
        largest_path = largest_path.max(total);
        largest_event = largest_event.max(value.largest_event);
        max_interactions = max_interactions.max(value.interactions);
        add_path(&mut sums[index % BATCHES], &value);
        if sums[index % BATCHES].iter().any(|sum| !sum.is_finite()) {
            return Err(Unresolved {
                completed_paths: index,
                requested_paths: paths,
                ..Unresolved::new(Reason::NonFinite)
            });
        }
    }
    let count = (paths / BATCHES) as f64;
    let batch_means = std::array::from_fn(|batch| sums[batch][..6].iter().sum::<f64>() / count);
    Ok(Stats {
        radiance: summarize(batch_means),
        by_order: std::array::from_fn(|order| {
            summarize(std::array::from_fn(|batch| sums[batch][order + 1] / count))
        }),
        atmosphere: summarize(std::array::from_fn(|batch| sums[batch][6] / count)),
        ground: summarize(std::array::from_fn(|batch| sums[batch][7] / count)),
        direct_ground: summarize(std::array::from_fn(|batch| sums[batch][0] / count)),
        ground_vertices: summarize(std::array::from_fn(|batch| sums[batch][8] / count)),
        direct_volume: summarize(std::array::from_fn(|batch| sums[batch][9] / count)),
        zero_batches: batch_means.iter().filter(|&&value| value == 0.0).count(),
        batch_means,
        nonzero_paths,
        nonzero_by_order,
        largest_path,
        largest_event,
        max_interactions,
        paths,
        seed,
    })
}

fn add_path(sum: &mut [f64; 10], path: &PathValue) {
    for (sum, value) in sum[..6].iter_mut().zip(path.orders) {
        *sum += value;
    }
    sum[6] += path.atmosphere;
    sum[7] += path.ground;
    sum[8] += path.ground_vertices;
    sum[9] += path.direct_volume;
}

fn summarize(batches: [f64; BATCHES]) -> Estimate {
    let mean = batches.iter().sum::<f64>() / BATCHES as f64;
    let variance = batches
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (BATCHES * (BATCHES - 1)) as f64;
    Estimate {
        mean,
        standard_error: variance.sqrt(),
    }
}

mod tests;
