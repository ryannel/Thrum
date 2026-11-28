//! CPU puff geometry and point density. The column integral lives alongside them.
mod column;
#[cfg(test)]
mod tests;

use crate::{Density, Field, PuffProfile, State, Type, smooth};
use std::cell::RefCell;
use std::f64::consts::{PI, TAU};

#[derive(Clone, Copy)]
struct Puff {
    centre: [f64; 3],
    radius: f64,
}

impl Puff {
    fn horizontal_distance_squared(self, air: [f64; 2]) -> f64 {
        (air[0] - self.centre[0]).powi(2) + (air[1] - self.centre[2]).powi(2)
    }
}

struct Cluster {
    centre: [f64; 2],
    reach: f64,
    top: f64,
    age: f64,
    strength: f64,
    offset: f64,
    puffs: Vec<Puff>,
}

impl Cluster {
    fn contains_column(&self, air: [f64; 2]) -> bool {
        (air[0] - self.centre[0]).powi(2) + (air[1] - self.centre[1]).powi(2) < self.reach.powi(2)
    }

    fn profile(&self, air: [f64; 2], height: f64, fade: f64) -> f64 {
        self.puffs
            .iter()
            .map(|puff| {
                let centre = puff.centre[1] + self.offset;
                let distance_squared =
                    puff.horizontal_distance_squared(air) + (height - centre).powi(2);
                interior(puff.radius, distance_squared)
            })
            .fold(0.0, f64::max)
            .min(height)
            .min(fade)
            / fade
    }
}

fn interior(radius: f64, distance_squared: f64) -> f64 {
    let inside = radius.powi(2) - distance_squared;
    if inside > 0.0 {
        inside / (radius + distance_squared.sqrt())
    } else {
        0.0
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Key {
    seed: u32,
    pitch: f64,
    period: f64,
    depth: [f64; 2],
    fade: f64,
    site: [i64; 2],
    time: f64,
    coverage: f64,
    share: f64,
}

struct CachedClusters {
    key: Key,
    clusters: Vec<Cluster>,
}

const CACHE_SLOTS: u16 = 64;
thread_local! {
    // A ray or vertical integration repeatedly visits the same neighbourhood.
    // Only the immutable result of the weather function is cached.
    static CLUSTERS: RefCell<[Option<CachedClusters>; CACHE_SLOTS as usize]> =
        const { RefCell::new([const { None }; CACHE_SLOTS as usize]) };
}

impl Field {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "site coordinates are floors"
    )]
    fn with_clusters<R>(
        &self,
        air: [f64; 2],
        time: f64,
        coverage: f64,
        share: f64,
        fade: f64,
        evaluate: impl FnOnce(&[Cluster]) -> R,
    ) -> R {
        if coverage <= 0.0 || share <= 0.0 {
            return evaluate(&[]);
        }
        // The heaped type supplies mature geometry. Type changes move the
        // same spheres vertically by changing the cell share.
        let depth = self.types[self.types.len() - 1].depth;
        let site = air.map(|axis| (axis / self.profile.pitch).floor() as i64);
        let key = Key {
            seed: self.seed,
            pitch: self.profile.pitch,
            period: self.profile.period,
            depth,
            fade,
            site,
            time,
            coverage,
            share,
        };
        let slot = usize::try_from(
            site[0]
                .wrapping_add(17_i64.wrapping_mul(site[1]))
                .rem_euclid(i64::from(CACHE_SLOTS)),
        )
        .expect("nonnegative cache slot");
        CLUSTERS.with_borrow_mut(|cache| {
            let entry = &mut cache[slot];
            if entry.as_ref().is_none_or(|cached| cached.key != key) {
                *entry = Some(CachedClusters {
                    key,
                    clusters: self.clusters(key),
                });
            }
            evaluate(&entry.as_ref().expect("populated cache entry").clusters)
        })
    }

    #[expect(clippy::cast_possible_truncation, reason = "generation is a floor")]
    fn clusters(&self, key: Key) -> Vec<Cluster> {
        let mut clusters = Vec::new();
        for dx in -1..=1 {
            for dz in -1..=1 {
                let i = key.site[0].wrapping_add(dx);
                let j = key.site[1].wrapping_add(dz);
                let generation = (key.time / key.period - self.random(i, j, 0, 1)).floor() as i64;
                for g in [generation.wrapping_sub(1), generation] {
                    let cell = self.cell(i, j, g);
                    let age = (key.time - cell.birth) / cell.life;
                    let selected =
                        smooth((0.95 * smooth(key.coverage / 0.6) - cell.threshold) / 0.15);
                    let envelope = (PI * age).sin().powi(2) * selected * key.share;
                    if envelope == 0.0 || !(0.0..1.0).contains(&age) {
                        continue;
                    }
                    clusters.push(self.cluster(cell, age, envelope, key.depth, key.fade));
                }
            }
        }
        clusters
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounded nonnegative hash counts"
    )]
    fn cluster(
        &self,
        cell: crate::Cell,
        age: f64,
        strength: f64,
        depth: [f64; 2],
        fade: f64,
    ) -> Cluster {
        let [i, j, g] = cell.identity;
        let hash = |thermal: u32, level: u32, stream: u32| {
            self.random(i, j, g, 100 + thermal * 2048 + level * 16 + stream)
        };
        let count = 2 * (5 + (4.0 * hash(0, 0, 1)) as u32);
        let width = cell.radius.min(self.profile.pitch * 0.60);
        let depth = depth[0] + (depth[1] - depth[0]) * cell.depth;
        let mut puffs = Vec::new();
        for thermal in 0..count {
            let h = |level, stream| hash(thermal + 1, level, stream);
            let rho = 0.56 * width * h(0, 2).sqrt();
            let (s, c) = (TAU * h(0, 3)).sin_cos();
            let reach =
                depth * (1.0 - (rho / width).powi(2)).powf(0.8) * (0.4 + 0.6 * h(0, 4).powf(1.5));
            let size = width * (0.26 + 0.2 * h(0, 5));
            // Small, shallow thermals still have room for several levels.
            let mut radius = size.max(0.4 * fade).min(2.0 * fade).min(0.55 * reach);
            // Keep rounded shoulders where neighbouring thermals meet the base.
            let root_height = (0.35 + 0.30 * h(0, 14)) * radius;
            let mut centre = [rho * c, root_height, rho * s];
            for level in 0..=4 {
                puffs.push(Puff { centre, radius });
                for child in 0..4 + (3.0 * h(level, 6)) as u32 {
                    let sample = level * 8 + child + 8;
                    let (ps, pc) = (25.0 + 80.0 * h(sample, 7)).to_radians().sin_cos();
                    let (ts, tc) = (TAU * h(sample, 8)).sin_cos();
                    let distance = 0.85 * radius;
                    puffs.push(Puff {
                        centre: [
                            centre[0] + distance * ps * tc,
                            centre[1] + distance * pc,
                            centre[2] + distance * ps * ts,
                        ],
                        radius: radius * (0.36 + 0.24 * h(sample, 9)),
                    });
                }
                if centre[1] + radius >= reach {
                    break;
                }
                let (s, c) = (TAU * h(level, 10)).sin_cos();
                let lean = 0.7 * radius * h(level, 11);
                centre[0] += lean * c;
                centre[1] += radius * (0.7 + 0.3 * h(level, 12));
                centre[2] += lean * s;
                radius *= 0.76 + 0.16 * h(level, 13);
            }
        }
        // Fit the whole cluster around its condensation base so the stacks
        // and their shoulders keep the same proportions.
        let reach = puffs
            .iter()
            .map(|p| p.centre[0].hypot(p.centre[2]) + p.radius)
            .fold(0.0, f64::max);
        let top = puffs
            .iter()
            .map(|p| p.centre[1] + p.radius)
            .fold(0.0, f64::max);
        let scale = 1.0_f64.min(cell.radius / reach).min(depth / top);
        for puff in &mut puffs {
            puff.radius *= scale;
            puff.centre[0] = cell.centre[0] + puff.centre[0] * scale;
            puff.centre[1] *= scale;
            puff.centre[2] = cell.centre[1] + puff.centre[2] * scale;
        }
        let top = top * scale;
        let offset = (strength - 1.0) * top;
        Cluster {
            centre: cell.centre,
            reach: reach * scale,
            top: top + offset,
            age,
            strength,
            offset,
            puffs,
        }
    }

    pub(super) fn puff_density(
        &self,
        point: [f64; 3],
        state: &State,
        kind: Type,
        fidelity: Density,
        puff: PuffProfile,
    ) -> f64 {
        let parameters = state.parameters();
        let height = point[1] - kind.base;
        if height <= 0.0 || parameters.coverage == 0.0 {
            return 0.0;
        }
        let air = [point[0], point[2]];
        let extinction = kind.extinction();
        let eroding = !matches!(fidelity, Density::Uneroded);
        let heaped = self.with_clusters(
            air,
            state.time(),
            parameters.coverage,
            (1.0 - kind.deck) * state.procedural_share(),
            puff.fade,
            |clusters| {
                clusters
                    .iter()
                    .filter(|cluster| height < cluster.top && cluster.contains_column(air))
                    .map(|cluster| cluster.strength * cluster.profile(air, height, puff.fade))
                    .fold(0.0, f64::max)
            },
        );
        let heaped_density = if heaped > 0.0 && eroding && puff.erosion > 0.0 {
            let threshold = self.puff_threshold(point, fidelity, puff.erosion);
            firm((heaped - threshold) / (1.0 - threshold))
        } else {
            firm(heaped)
        };
        let deck = if kind.deck > 0.0 {
            let (coverage, depth) = self.deck(air, state.time(), parameters, kind);
            let sigma = extinction * column::Deck::new(coverage, depth, kind.profile).at(height);
            if eroding && self.profile.erosion > 0.0 && sigma > 0.0 {
                let detail = match fidelity {
                    Density::Full => self.detail_noise(point),
                    _ => self.profile.detail_mean,
                };
                self.erode(sigma, extinction, self.shape_noise(point), detail, false)
                    * self.bank_shape(point, height / depth, kind.deck)
            } else {
                sigma
            }
        } else {
            0.0
        };
        (extinction * heaped_density).max(deck)
    }

    fn puff_threshold(&self, point: [f64; 3], fidelity: Density, erosion: f64) -> f64 {
        let coarse = 1.0 - self.shape_noise(point.map(|axis| axis * 1.45));
        let fine = if matches!(fidelity, Density::Full) {
            let offset = [811.0, 243.0, 1601.0];
            let fine_point = std::array::from_fn(|i| point[i] * 1.45 * 4.3 + offset[i]);
            1.0 - self.shape_noise(fine_point)
        } else {
            self.profile.detail_mean
        };
        erosion * (0.72 * coarse + 0.28 * fine)
    }
}

fn firm(profile: f64) -> f64 {
    profile.clamp(0.0, 1.0).powf(0.6)
}
