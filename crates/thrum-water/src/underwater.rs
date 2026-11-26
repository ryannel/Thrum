use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec3Swizzles, Vec4};
use thrum_render::contribution::ContributionFrame;
use thrum_sea_model::{LongWaves, Spectrum};
use thrum_water_model::{Field, Map};

const LENS_SIDE: usize = 33;
const SMOOTH_SIDE: usize = 9;
pub(super) const DRAW: &str = "water through lens";

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Lens {
    pub(super) state: [u32; 4], // gather required, columns wide and high, reserved
    columns: [[[f32; 4]; LENS_SIDE]; LENS_SIDE],
}

pub(super) struct Underwater {
    field: Field,
    long_amplitude: f64,
    short_amplitude: f64,
}

impl Underwater {
    pub(super) fn new(long: LongWaves, spectrum: &Spectrum) -> Self {
        let long_amplitude = spectrum.long.iter().map(|c| c.amplitude).sum();
        let short_amplitude = spectrum
            .cascades
            .iter()
            .flat_map(thrum_sea_model::Cascade::components)
            .map(|c| c.amplitude)
            .sum();
        let empty = || LongWaves::new(Vec::new(), 0.0).expect("empty wave band");
        Self {
            field: Field {
                layers: [long, empty(), empty(), empty()],
            },
            long_amplitude,
            short_amplitude,
        }
    }

    pub(super) fn lens(&self, frame: &ContributionFrame, map: &Map, seconds: f64) -> Lens {
        let plane = Plane::new(frame);
        let mut lens = Lens::zeroed();
        let may_fold = self.sample(&mut lens, &plane, map, seconds, SMOOTH_SIDE);
        if may_fold {
            self.sample(&mut lens, &plane, map, seconds, LENS_SIDE);
        }
        lens
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
                let bound = sample.row.gains[0] * self.long_amplitude + self.short_amplitude;
                let plain = map.surf_profile().is_none() || sample.bed.is_none();
                if plain && sample.row.level + bound < plane.lowest {
                    continue;
                }
                let roof = if plain && sample.row.level - bound > plane.highest.max(bed) {
                    sample.row.level
                } else {
                    let Ok(surface) = map.surface_on_row(&self.field, xz, seconds, sample) else {
                        continue;
                    };
                    surface.sample.height
                };
                *column = [roof as f32, bed as f32, 0.0, 1.0];
                let lowest = if height == 1 || width == 1 {
                    plane.lowest
                } else {
                    f64::from(p.y)
                };
                if roof + self.short_amplitude > lowest.max(bed) {
                    lens.state[0] = 1;
                }
                if !plain
                    && side == SMOOTH_SIDE
                    && (plane.lowest - self.short_amplitude..=plane.highest + self.short_amplitude)
                        .contains(&roof)
                {
                    may_fold = true;
                }
            }
        }
        may_fold
    }
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
}
