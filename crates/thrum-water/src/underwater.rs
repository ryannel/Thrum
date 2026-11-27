use glam::{Mat4, Vec3, Vec3Swizzles, Vec4};
use thrum_render::contribution::ContributionFrame;
use thrum_sea_model::{LongWaves, Spectrum};
use thrum_water_model::Map;

pub(crate) mod cut_gpu;
mod rest;
mod short;
pub(super) use rest::{RestMesh, Vertices};
use short::ShortBand;
pub(super) use short::ShortWaveDomain;

pub(super) const DRAW: &str = "water through lens";

pub(super) struct Underwater {
    long: LongWaves,
    long_amplitude: f64,
    heights: [f32; 4],
    short: [ShortBand; 3],
}

pub(super) struct Admission<'a> {
    pub(super) mesh: &'a RestMesh,
    pub(super) horizontal: f64,
    pub(super) transformed_ceiling: Option<f64>,
    pub(super) wake: Option<&'a super::interaction::Interaction>,
    pub(super) rain: bool,
    pub(super) domain: ShortWaveDomain,
    pub(super) level: f64,
}

impl Underwater {
    pub(super) fn new(long: LongWaves, spectrum: &Spectrum, heights: [f32; 4]) -> Self {
        let long_amplitude = spectrum.long.iter().map(|c| c.amplitude).sum();
        Self {
            long,
            long_amplitude,
            heights,
            short: spectrum.cascades.each_ref().map(ShortBand::new),
        }
    }

    pub(super) fn ambient_reach(&self, seconds: f64) -> f64 {
        let long = self.long_amplitude * self.long.choppiness();
        let short = self
            .short
            .iter()
            .map(|band| band.horizontal_bound(seconds) * band.geometry_weight(0.25))
            .sum::<f64>();
        (long + short).mul_add(1.05, 0.05)
    }

    pub(super) fn admitted(
        &mut self,
        frame: &ContributionFrame,
        map: &Map,
        seconds: f64,
        admission: &Admission<'_>,
    ) -> bool {
        let plane = Plane::new(frame);
        let rest = plane.rest_box(admission.mesh, frame.camera, admission.horizontal);
        rest.is_some_and(|rest| {
            // The displayed rain field can retain ring height between evaluations.
            admission.rain
                || (map.flow_bands().is_some() && intersects_river(map, rest))
                || admission
                    .wake
                    .is_some_and(|wake| wake.live_wake_intersects(rest))
                || !self.proves_dry(&plane, map, frame.camera, rest, seconds, admission)
        })
    }

    fn proves_dry(
        &mut self,
        plane: &Plane,
        map: &Map,
        camera: [f32; 3],
        [lo, hi]: [[f64; 2]; 2],
        seconds: f64,
        admission: &Admission<'_>,
    ) -> bool {
        if !lo.into_iter().chain(hi).all(f64::is_finite) {
            return false;
        }
        let centre = std::array::from_fn(|i| lo[i].midpoint(hi[i]));
        let radius = std::array::from_fn(|i| (hi[i] - lo[i]) * 0.5);
        if map.surf_profile().is_some() && !ShortWaveDomain::offshore(map, centre, radius) {
            return admission
                .transformed_ceiling
                .is_some_and(|upper| upper < plane.lowest);
        }
        let Some(gains) = admission
            .domain
            .gain_ranges(map, centre, radius, self.heights)
        else {
            return false;
        };
        let (level, fine) = admission
            .domain
            .still_level_and_spacing(map, admission.level);
        let camera = [camera[0], camera[2]].map(f64::from);
        // Within the innermost sea ring every resolved vertex uses the 1 m
        // long-wave cache and never blends with a coarser edge node.
        let near = fine && (0..2).all(|i| (centre[i] - camera[i].floor()).abs() + radius[i] < 7.5);
        let long = if near && gains[0][1] > 0.0 {
            let waves = &self.long;
            let taps = |axis: usize| {
                let first = (lo[axis] - 0.5).floor();
                let last = (hi[axis] - 0.5).floor() + 1.0;
                (0..17)
                    .map(move |i| first + f64::from(i))
                    .take_while(move |&v| v <= last)
            };
            let upper = taps(0)
                .flat_map(|x| taps(1).map(move |z| [x + 0.5, z + 0.5]))
                .map(|tap| {
                    waves
                        .components()
                        .iter()
                        .filter(|wave| {
                            let [x, z] = wave.wave_vector;
                            x.hypot(z) <= std::f64::consts::PI
                        })
                        .map(|wave| wave.amplitude * wave.anchored_phase(tap, seconds).cos())
                        .sum::<f64>()
                })
                .fold(f64::NEG_INFINITY, f64::max);
            scaled_upper(upper + 0.001 * self.long_amplitude, gains[0])
        } else {
            self.long_amplitude * gains[0][1]
        };
        let short = self
            .short
            .iter_mut()
            .zip(&gains[1..])
            .filter(|(_, gain)| gain[1] > 0.0)
            .map(|(band, gain)| {
                let upper = if near {
                    band.height_bound(centre, radius, seconds)
                } else {
                    band.amplitude * 1.001
                };
                // The shortest bands can be faded out on this mesh. A
                // negative crest bound only helps when every draw node uses it.
                let spacing = if upper < 0.0 && near { 0.5 } else { 0.25 };
                scaled_upper(upper, *gain) * band.geometry_weight(spacing)
            })
            .sum::<f64>();
        let roof = level + long + short;
        let rounding = f64::from(f32::EPSILON) * 16.0 * roof.abs().max(plane.lowest.abs()).max(1.0);
        roof + rounding < plane.lowest
    }
}

// Near a flowing chart the cut must read the submitted crest. The
// wind-only roof cannot bound a further independently phased field.
fn intersects_river(map: &Map, [lo, hi]: [[f64; 2]; 2]) -> bool {
    (0..super::map_gpu::BODY_LIMIT).any(|id| {
        map.river_of(id).is_some()
            && map.surface_bounds(id).is_some_and(|[bottom, top]| {
                (0..2).all(|axis| bottom[axis] <= hi[axis] && top[axis] >= lo[axis])
            })
    })
}

fn scaled_upper(value: f64, [lo, hi]: [f64; 2]) -> f64 {
    if value < 0.0 { lo * value } else { hi * value }
}

pub(super) fn cascade_geometry_weight(length: f64, spacing: f64) -> f64 {
    let ratio = ((spacing * 128.0 / length - 1.0) / 3.0).clamp(0.0, 1.0);
    1.0 - ratio * ratio * (3.0 - 2.0 * ratio)
}

struct Plane {
    inverse: Mat4,
    corners: [Vec3; 4],
    lowest: f64,
}

impl Plane {
    fn new(frame: &ContributionFrame) -> Self {
        let mut plane = Self {
            inverse: Mat4::from_cols_array_2d(&frame.view_proj).inverse(),
            corners: [Vec3::ZERO; 4],
            lowest: f64::INFINITY,
        };
        plane.corners = [
            plane.point(-1.0, -1.0),
            plane.point(1.0, -1.0),
            plane.point(-1.0, 1.0),
            plane.point(1.0, 1.0),
        ];
        for p in plane.corners {
            plane.lowest = plane.lowest.min(f64::from(p.y));
        }
        plane
    }

    fn point(&self, x: f32, y: f32) -> Vec3 {
        let h = self.inverse * Vec4::new(x, y, 1.0, 1.0);
        h.truncate() / h.w
    }

    fn horizontal_box(&self) -> ([f64; 2], [f64; 2]) {
        let [low, high] = self.bounds();
        (
            std::array::from_fn(|i| low[i].midpoint(high[i])),
            std::array::from_fn(|i| (high[i] - low[i]) * 0.5),
        )
    }

    fn bounds(&self) -> [[f64; 2]; 2] {
        let low = self
            .corners
            .into_iter()
            .fold(Vec3::splat(f32::INFINITY), Vec3::min);
        let high = self
            .corners
            .into_iter()
            .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
        [low, high].map(|v| v.xz().to_array().map(f64::from))
    }

    fn rest_box(
        &self,
        mesh: &RestMesh,
        camera: [f32; 3],
        horizontal: f64,
    ) -> Option<[[f64; 2]; 2]> {
        let (centre, radius) = self.horizontal_box();
        if !horizontal.is_finite() || !centre.into_iter().chain(radius).all(f64::is_finite) {
            return Some([[f64::NEG_INFINITY; 2], [f64::INFINITY; 2]]);
        }
        let anchor = [camera[0].floor(), camera[2].floor()].map(f64::from);
        let reach = radius.map(|span| span + horizontal);
        mesh.bounds(
            [
                std::array::from_fn(|axis| centre[axis] - reach[axis]),
                std::array::from_fn(|axis| centre[axis] + reach[axis]),
            ],
            anchor,
        )
    }
}
