use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec3Swizzles, Vec4};
use thrum_render::contribution::ContributionFrame;
use thrum_sea_model::{Cascade, LongWaves, Spectrum};
use thrum_water_model::{Field, Map, MapSample, SurfSheet, Surface, WaterError, WholeCut};

mod short;
use short::ShortBand;
pub(super) use short::ShortWaveDomain;

const LENS_SIDE: usize = 33;
const SMOOTH_SIDE: usize = 9;
const MAX_STRIP_SAMPLES: usize = 384;
pub(super) const DRAW: &str = "water through lens";

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Lens {
    pub(super) state: [u32; 4], // gather required, columns wide and high, adaptive strip
    // Roof and bed in metres, strip coordinate, and short-wave weight plus one.
    // The fourth component is zero when there is no owning water body.
    columns: [[[f32; 4]; LENS_SIDE]; LENS_SIDE],
}

pub(super) struct Underwater {
    field: Field,
    long_amplitude: f64,
    variances: [f64; 4],
    heights: [f32; 4],
    short: [ShortBand; 3],
}

impl Underwater {
    pub(super) fn new(long: LongWaves, spectrum: &Spectrum, heights: [f32; 4]) -> Self {
        let long_amplitude = spectrum.long.iter().map(|c| c.amplitude).sum();
        let long_variance = spectrum
            .long
            .iter()
            .map(|c| 0.5 * c.amplitude.powi(2))
            .sum();
        let [a, b, c] = spectrum.cascades.each_ref().map(Cascade::variance);
        let empty = || LongWaves::new(Vec::new(), 0.0).expect("empty wave band");
        Self {
            field: Field {
                layers: [long, empty(), empty(), empty()],
            },
            long_amplitude,
            variances: [long_variance, a, b, c],
            heights,
            short: spectrum.cascades.each_ref().map(ShortBand::new),
        }
    }

    pub(super) fn lens(
        &mut self,
        frame: &ContributionFrame,
        map: &Map,
        seconds: f64,
        short_domain: Option<ShortWaveDomain>,
    ) -> Lens {
        let plane = Plane::new(frame);
        let mut lens = Lens::zeroed();
        let may_fold = self.sample(&mut lens, &plane, map, seconds, SMOOTH_SIDE);
        if may_fold
            && self
                .level_strip(&plane, map, seconds, frame.render_extent, &mut lens)
                .is_none()
        {
            self.sample(&mut lens, &plane, map, seconds, LENS_SIDE);
        }
        if lens.state[0] != 0
            && let Some(domain) = short_domain
            && self.short.iter().any(|band| band.amplitude > 0.0)
            && self.proves_dry(&lens, &plane, map, domain, seconds)
        {
            lens.state[0] = 0;
        }
        lens
    }

    fn proves_dry(
        &mut self,
        lens: &Lens,
        plane: &Plane,
        map: &Map,
        domain: ShortWaveDomain,
        seconds: f64,
    ) -> bool {
        let lowest = plane.lowest;
        let (roof, weight) = lens
            .columns
            .iter()
            .flatten()
            .filter(|column| column[3] > 0.0)
            .fold(
                (f64::NEG_INFINITY, [2.0_f32, 0.0_f32]),
                |(roof, [lo, hi]), column| {
                    (
                        roof.max(f64::from(column[0])),
                        [lo.min(column[3]), hi.max(column[3])],
                    )
                },
            );
        let rounding = f64::from(f32::EPSILON) * 8.0 * roof.abs().max(lowest.abs()).max(1.0);
        // Interpolation leaves the short-wave weight at zero when all
        // occupied lens columns have lost their short waves in the swash.
        if weight[1] <= 1.0 {
            return roof.is_finite() && roof + rounding < lowest;
        }
        let weight = weight.map(|w| f64::from(w - 1.0));
        let (centre, radius) = plane.horizontal_box();
        let Some(gains) = domain.gain_ranges(map, centre, radius, self.heights) else {
            return false;
        };
        let bound = weight[1] * self.short_bound(&gains.map(|range| range[1]));
        if bound == 0.0 || roof - bound >= lowest {
            return false;
        }
        let short = self
            .short
            .iter_mut()
            .zip(&gains[1..])
            .filter(|(_, gain)| gain[1] > 0.0)
            .map(|(band, gain)| scaled_upper(band.height_bound(centre, radius, seconds), *gain))
            .sum::<f64>();
        roof + scaled_upper(short, weight) + rounding < lowest
    }

    fn surf_sheet<'a>(
        &'a self,
        map: &'a Map,
        body: usize,
        seconds: f64,
    ) -> Result<SurfSheet<'a>, WaterError> {
        let [_, a, b, c] = self.variances;
        Ok(map
            .surf_sheet(&self.field, body, seconds)?
            .with_short_wave_variances([a, b, c]))
    }

    fn surface_on_row(
        &self,
        map: &Map,
        point: [f64; 2],
        seconds: f64,
        row: MapSample,
    ) -> Result<Surface, WaterError> {
        if map.surf_profile().is_some() && row.bed.is_some() {
            return self
                .surf_sheet(map, row.body, seconds)?
                .surface_on_row(point, row);
        }
        map.surface_on_row(&self.field, point, seconds, row)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "slice limits and projected GPU metres are bounded"
    )]
    fn level_strip(
        &self,
        plane: &Plane,
        map: &Map,
        seconds: f64,
        [width, height]: [u32; 2],
        lens: &mut Lens,
    ) -> Option<()> {
        if width < LENS_SIDE as u32
            || plane.corners[0].xz() != plane.corners[2].xz()
            || plane.corners[0].xz() == plane.corners[1].xz()
        {
            return None;
        }
        let profile = map.surf_profile()?;
        let xz = |x| {
            let uv = (x as f32 + 0.5) / width as f32;
            let p = plane.point(uv.mul_add(2.0, -1.0), 0.0);
            [f64::from(p.x), f64::from(p.z)]
        };
        let last = width - 1;
        let anchors: Vec<_> = (0..LENS_SIDE as u32)
            .map(|i| i * last / (LENS_SIDE as u32 - 1))
            .collect();
        let world: Vec<_> = anchors.iter().copied().map(xz).collect();
        let body = map.sample(xz(last / 2))?.body;
        let sheet = self.surf_sheet(map, body, seconds).ok()?;
        let a = world[0];
        let b = world[world.len() - 1];
        let prepared = WholeCut::new(&sheet, &world, profile.cut)
            .ok()?
            .with_projection(
                std::array::from_fn(|i| a[i].min(b[i])),
                std::array::from_fn(|i| a[i].max(b[i])),
            );
        let mut cached = vec![None; width as usize];
        let mut budget = MAX_STRIP_SAMPLES;
        let mut under = false;
        let mut query = |x| {
            let slot = &mut cached[x as usize];
            if let Some(value) = *slot {
                return Some(value);
            }
            budget = budget.checked_sub(1)?;
            let position = xz(x);
            let uv = (x as f32 + 0.5) / width as f32;
            let value = if let Some(row) = map.sample(position) {
                let result = if row.body == body && row.bed.is_some() {
                    sheet
                        .surface_with_cut(position, row, &prepared)
                        .or_else(|_| sheet.surface_on_row(position, row))
                } else {
                    self.surface_on_row(map, position, seconds, row)
                }
                .ok()?;
                let roof = result.sample.height;
                let bed = row.bed.unwrap_or(-1e30);
                under |= roof + self.short_bound(&row.row.gains) > plane.lowest.max(bed);
                let weight = map.surf_short_wave_weight(position, row, self.variances, roof);
                [roof as f32, bed as f32, uv, 1.0 + weight as f32]
            } else {
                [-1e30, -1e30, uv, 0.0]
            };
            *slot = Some(value);
            Some(value)
        };
        let tolerance = ((plane.highest - plane.lowest) / f64::from(height) * 0.25) as f32;
        let short_amplitude = self.short.iter().map(|band| band.amplitude).sum::<f64>() as f32;
        let mut samples = Vec::with_capacity(LENS_SIDE * 2);
        samples.push(query(0)?);
        for span in anchors.windows(2) {
            let a = (span[0], query(span[0])?);
            let b = (span[1], query(span[1])?);
            refine_strip(a, b, tolerance, short_amplitude, &mut query, &mut samples)?;
        }
        lens.state = [u32::from(under), samples.len() as u32, 1, 1];
        for (column, sample) in lens.columns.iter_mut().flatten().zip(samples) {
            *column = sample;
        }
        Some(())
    }

    fn short_bound(&self, gains: &[f64; 4]) -> f64 {
        self.short
            .iter()
            .zip(&gains[1..])
            .map(|(band, gain)| band.amplitude * gain)
            .sum()
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "lens grid indices and GPU metres"
    )]
    fn sample(&self, lens: &mut Lens, plane: &Plane, map: &Map, seconds: f64, side: usize) -> bool {
        // A level near plane repeats the same world columns down the image.
        // Its height stays per-pixel, but we only cut those columns once.
        let width = if plane.corners[0].xz() == plane.corners[1].xz() {
            1
        } else {
            side
        };
        let height = if plane.corners[0].xz() == plane.corners[2].xz() {
            1
        } else {
            side
        };
        lens.state = [0, width as u32, height as u32, 0];
        let mut may_fold = false;
        for (y, row) in lens.columns[..height].iter_mut().enumerate() {
            for (x, column) in row[..width].iter_mut().enumerate() {
                let p = plane.point(lens_coordinate(x, width), -lens_coordinate(y, height));
                *column = [-1e30, -1e30, 0.0, 0.0];
                let xz = [f64::from(p.x), f64::from(p.z)];
                let Some(sample) = map.sample(xz) else {
                    continue;
                };
                let bed = sample.bed.unwrap_or(-1e30);
                // The surface can attenuate these gains further in shallow
                // water. Open-sea detail cannot admit a sheltered lens.
                let short_bound = self.short_bound(&sample.row.gains);
                let bound = sample.row.gains[0] * self.long_amplitude + short_bound;
                let plain = map.surf_profile().is_none() || sample.bed.is_none();
                if plain && sample.row.level + bound < plane.lowest {
                    continue;
                }
                let roof = if plain && sample.row.level - bound > plane.highest.max(bed) {
                    sample.row.level
                } else {
                    let Ok(surface) = self.surface_on_row(map, xz, seconds, sample) else {
                        continue;
                    };
                    surface.sample.height
                };
                let weight = map.surf_short_wave_weight(xz, sample, self.variances, roof);
                *column = [roof as f32, bed as f32, 0.0, 1.0 + weight as f32];
                let lowest = if height == 1 || width == 1 {
                    plane.lowest
                } else {
                    f64::from(p.y)
                };
                if roof + short_bound > lowest.max(bed) {
                    lens.state[0] = 1;
                }
                if !plain
                    && side == SMOOTH_SIDE
                    && (plane.lowest - short_bound..=plane.highest + short_bound).contains(&roof)
                {
                    may_fold = true;
                }
            }
        }
        may_fold
    }
}

fn scaled_upper(value: f64, [lo, hi]: [f64; 2]) -> f64 {
    if value < 0.0 { lo * value } else { hi * value }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "render extents are small enough for exact indices"
)]
fn refine_strip(
    (first, a): (u32, [f32; 4]),
    (last, b): (u32, [f32; 4]),
    tolerance: f32,
    short_amplitude: f32,
    query: &mut impl FnMut(u32) -> Option<[f32; 4]>,
    output: &mut Vec<[f32; 4]>,
) -> Option<()> {
    let span = last - first;
    if span > 1 {
        let mut refine = false;
        for part in 1..=3 {
            let x = first + (span * part / 4).max(1);
            let p = query(x)?;
            let t = (x - first) as f32 / span as f32;
            let error = |axis: usize| (p[axis] - (b[axis] - a[axis]).mul_add(t, a[axis])).abs();
            refine |= (p[3] > 0.0) != (a[3] > 0.0)
                || (p[3] > 0.0) != (b[3] > 0.0)
                || error(0) + short_amplitude * error(3) > tolerance
                || error(1) > tolerance;
        }
        if refine {
            let middle = first + span / 2;
            let centre = (middle, query(middle)?);
            refine_strip(
                (first, a),
                centre,
                tolerance,
                short_amplitude,
                query,
                output,
            )?;
            return refine_strip(centre, (last, b), tolerance, short_amplitude, query, output);
        }
    }
    output.push(b);
    Some(())
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "lens grid is at most 33 points wide"
)]
fn lens_coordinate(i: usize, size: usize) -> f32 {
    if size == 1 {
        return 0.0;
    }
    f32::from(i as u8).mul_add(2.0 / f32::from((size - 1) as u8), -1.0)
}

struct Plane {
    inverse: Mat4,
    corners: [Vec3; 4],
    lowest: f64,
    highest: f64,
}

impl Plane {
    fn new(frame: &ContributionFrame) -> Self {
        let mut plane = Self {
            inverse: Mat4::from_cols_array_2d(&frame.view_proj).inverse(),
            corners: [Vec3::ZERO; 4],
            lowest: f64::INFINITY,
            highest: f64::NEG_INFINITY,
        };
        plane.corners = [
            plane.point(-1.0, -1.0),
            plane.point(1.0, -1.0),
            plane.point(-1.0, 1.0),
            plane.point(1.0, 1.0),
        ];
        for p in plane.corners {
            plane.lowest = plane.lowest.min(f64::from(p.y));
            plane.highest = plane.highest.max(f64::from(p.y));
        }
        plane
    }

    fn point(&self, x: f32, y: f32) -> Vec3 {
        let h = self.inverse * Vec4::new(x, y, 1.0, 1.0);
        h.truncate() / h.w
    }

    fn horizontal_box(&self) -> ([f64; 2], [f64; 2]) {
        let low = self
            .corners
            .into_iter()
            .fold(Vec3::splat(f32::INFINITY), Vec3::min);
        let high = self
            .corners
            .into_iter()
            .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
        let low = low.xz().to_array().map(f64::from);
        let high = high.xz().to_array().map(f64::from);
        (
            std::array::from_fn(|i| low[i].midpoint(high[i])),
            std::array::from_fn(|i| (high[i] - low[i]) * 0.5),
        )
    }
}
