//! Bound the vertical integral over the union of the clipped sphere chords.
use super::{Cluster, firm, interior};
use crate::{Column, Field, Parameters, smooth};

pub(super) struct Deck {
    coverage: f64,
    depth: f64,
    fade: [f64; 2],
}

impl Deck {
    pub(super) fn new(coverage: f64, depth: f64, fade: [f64; 2]) -> Self {
        Self {
            coverage,
            depth,
            fade,
        }
    }

    pub(super) fn at(&self, height: f64) -> f64 {
        if height <= 0.0 || height >= self.depth {
            return 0.0;
        }
        let h = height / self.depth;
        let [rise, fall] = self.fade;
        self.coverage * smooth(h / rise) * smooth((1.0 - h) / fall)
    }

    fn support(&self, level: f64) -> Option<[f64; 2]> {
        if level >= self.coverage {
            return None;
        }
        let x = level / self.coverage;
        let inverse_smooth = 0.5 - ((1.0 - 2.0 * x).asin() / 3.0).sin();
        Some([
            self.depth * self.fade[0] * inverse_smooth,
            self.depth * (1.0 - self.fade[1] * inverse_smooth),
        ])
    }

    fn integral(&self) -> f64 {
        self.depth * self.coverage * (1.0 - self.fade[0].midpoint(self.fade[1]))
    }

    fn integrated_to(&self, height: f64) -> f64 {
        let h = (height / self.depth).clamp(0.0, 1.0);
        let [rise, fall] = self.fade;
        let integrated_smooth = |u: f64| u.powi(3) * (1.0 - 0.5 * u);
        let area = if h <= rise {
            rise * integrated_smooth(h / rise)
        } else if h >= 1.0 - fall {
            1.0 - rise.midpoint(fall) - fall * integrated_smooth((1.0 - h) / fall)
        } else {
            h - 0.5 * rise
        };
        area * self.depth * self.coverage
    }

    fn excess(&self, bottom: f64, top: f64, puff: f64) -> f64 {
        let mut area = puff * (top - bottom);
        if self.coverage > 0.0 {
            area -= self.integrated_to(top) - self.integrated_to(bottom);
            if let Some([deck_bottom, deck_top]) = self.support(puff) {
                let lo = bottom.max(deck_bottom);
                let hi = top.min(deck_top);
                if hi > lo {
                    area += self.integrated_to(hi) - self.integrated_to(lo) - puff * (hi - lo);
                }
            }
        }
        area.max(0.0)
    }
}

struct Chord {
    centre: f64,
    radius: f64,
    horizontal_squared: f64,
    half: f64,
    fade: f64,
    strength: f64,
}

impl Chord {
    fn peak_height(&self) -> f64 {
        let radial = self.half.powi(2) / (self.radius + self.horizontal_squared.sqrt());
        if radial <= self.centre {
            self.centre
        } else {
            // The level-base fade crosses the radial fade above the centre.
            (self.half - self.centre) * (self.half + self.centre)
                / (2.0 * (self.radius - self.centre))
        }
    }

    fn at(&self, height: f64) -> f64 {
        let distance_squared = self.horizontal_squared + (height - self.centre).powi(2);
        let radial = interior(self.radius, distance_squared);
        self.strength * (radial.min(height) / self.fade).clamp(0.0, 1.0)
    }

    fn bands(&self, peak: f64, bands: &mut Vec<Band>) {
        // Partition this exact chord independently, so even a grazing chord
        // has the same relative resolution. Each band covers the maximum
        // of a unimodal profile at the peak or the closest endpoint.
        const BANDS: u32 = 32;
        let bottom = (self.centre - self.half).max(0.0);
        let top = self.centre + self.half;
        let extent = top - bottom;
        let mut start = bottom;
        for i in 1..=BANDS {
            let end = bottom + extent * (f64::from(i) / f64::from(BANDS));
            if end > start {
                bands.push(Band {
                    start,
                    end,
                    profile: self.at(peak.clamp(start, end)),
                });
            }
            start = end;
        }
    }
}

struct Band {
    start: f64,
    end: f64,
    profile: f64,
}

impl Field {
    pub(in crate::field) fn puff_column(
        &self,
        air: [f64; 2],
        time: f64,
        parameters: Parameters,
        share: f64,
        clear: Column,
        fade: f64,
    ) -> Column {
        let kind = self.kind(parameters.kind);
        let (coverage, depth) = if kind.deck > 0.0 {
            self.deck(air, time, parameters, kind)
        } else {
            (0.0, 0.0)
        };
        let deck = Deck::new(coverage, depth, kind.profile);
        self.with_clusters(
            air,
            time,
            parameters.coverage,
            (1.0 - kind.deck) * share,
            fade,
            |clusters| evaluate_column(clusters, air, fade, &deck, kind.extinction(), clear),
        )
    }
}

fn evaluate_column(
    clusters: &[Cluster],
    air: [f64; 2],
    fade: f64,
    deck: &Deck,
    extinction: f64,
    mut column: Column,
) -> Column {
    let mut bands = Vec::new();
    let mut puff_coverage: f64 = 0.0;
    let mut height = deck.depth;
    for cluster in clusters.iter().filter(|c| c.contains_column(air)) {
        let mut coverage: f64 = 0.0;
        for puff in &cluster.puffs {
            let horizontal_squared = puff.horizontal_distance_squared(air);
            let half_squared = puff.radius.powi(2) - horizontal_squared;
            if half_squared <= 0.0 {
                continue;
            }
            let half = half_squared.sqrt();
            let centre = puff.centre[1] + cluster.offset;
            if centre + half <= 0.0 {
                continue;
            }
            let chord = Chord {
                centre,
                radius: puff.radius,
                horizontal_squared,
                half,
                fade,
                strength: cluster.strength,
            };
            let peak = chord.peak_height();
            coverage = coverage.max(chord.at(peak));
            height = height.max(centre + half);
            chord.bands(peak, &mut bands);
        }
        if coverage > puff_coverage {
            column.age = cluster.age;
            puff_coverage = coverage;
        }
    }
    column.coverage = puff_coverage.max(deck.coverage);
    column.top += height;
    column.optical_depth = extinction * upper_integral(&mut bands, deck);
    column
}

fn upper_integral(bands: &mut [Band], deck: &Deck) -> f64 {
    // Sweep the bands' endpoints, taking the largest active upper profile
    // exactly once over every interval. The deck keeps its exact integral
    // and contributes independently wherever it is greater than that bound.
    bands.sort_unstable_by(|a, b| a.start.total_cmp(&b.start));
    let mut active: Vec<&Band> = Vec::new();
    let mut next = bands.iter().peekable();
    let mut height = 0.0;
    let mut excess = 0.0;
    while next.peek().is_some() || !active.is_empty() {
        while next.peek().is_some_and(|band| band.start <= height) {
            active.extend(next.next());
        }
        active.retain(|band| band.end > height);
        let mut end = next.peek().map_or(f64::INFINITY, |band| band.start);
        let mut profile: f64 = 0.0;
        for band in &active {
            end = end.min(band.end);
            profile = profile.max(band.profile);
        }
        if profile > 0.0 {
            excess += deck.excess(height, end, firm(profile));
        }
        height = end;
    }
    // Allow for floating-point accumulation and intersection rounding.
    deck.integral() + excess * (1.0 + 1e-12)
}
