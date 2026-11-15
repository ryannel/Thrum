use thrum_water_model::{Bed, ShoreConfig, Source, Stage};

pub(crate) fn config() -> ShoreConfig {
    ShoreConfig {
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 2.0,
            value: 0.0,
        },
        margin: 1.5,
        floor: 0.5,
        fetch: 1400.0,
        seed: None,
        source: Source::Water,
    }
}

pub(crate) fn bed(origin: [f64; 2], h: f64, size: [usize; 2], f: impl Fn(f64, f64) -> f64) -> Bed {
    let heights = (0..size[1])
        .flat_map(|z| (0..size[0]).map(move |x| (x, z)))
        .map(|(x, z)| f(origin[0] + x as f64 * h, origin[1] + z as f64 * h))
        .collect();
    Bed::new(origin, h, size, heights).expect("analytic finite bed")
}

// Independent bisection of dispersion. No model solver or derivative is used.
pub(crate) fn wave_number(k0: f64, depth: f64) -> f64 {
    let (mut lo, mut hi) = (k0, k0 + (k0 / depth).sqrt() * 2.0);
    for _ in 0..60 {
        let mid = f64::midpoint(lo, hi);
        if mid * (mid * depth).tanh() > k0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    f64::midpoint(lo, hi)
}
