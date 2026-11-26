//! Shared authored fixtures for native and browser lens conformance.
use glam::{Mat4, Vec4};
use std::{
    error::Error,
    f64::consts::{FRAC_1_SQRT_2, TAU},
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, MeshPlacement, SkyGame};
use thrum_render::synthetic::SyntheticView;
use thrum_sea_model::{Cascade, Component, GRID_SIZE, LongWaves, Spectrum};
use thrum_water::{Optics, Profile, Water};
use thrum_water_model::{
    Bed, Body, BodyKind, Field, Impulse, Map, Sheet, SheetPoint, Stage, SurfProfile,
    highest_triangle, limit,
};

/// Simulation step shared by the wave probes.
pub const STEP: Duration = Duration::from_nanos(16_666_667);
/// Long-wave deformation used by the water and its model.
pub const CHOPPINESS: f64 = 0.5;
/// Vertical field of view of the level probe in radians.
pub const FOV: f32 = 0.85;
/// Vertical translation that puts the browser fixtures within the sky shell's camera range.
pub const BROWSER_DATUM: f64 = 8.0;
/// The four model fixtures submitted by the lens tests.
pub const NAMES: [&str; 4] = ["trade", "lake", "swash", "lip"];
/// Submitted grid used by the matching checks.
pub const EXTENT: [u32; 2] = [48, 27];
/// The same off-lens stone impulse used by both submitted witnesses.
pub const STONE: Impulse = Impulse {
    id: 1,
    tick: 2,
    place: [0.0, -1.5],
    vertical: 5000.0,
    radius: 0.58,
};

/// The plane beach and crossing wave trains used by the surf probes.
/// # Panics
/// Panics if the fixed model inputs are invalid.
#[must_use]
pub fn beach(datum: f64) -> (Map, Spectrum, Field) {
    let heights =
        (0..33).flat_map(|_| (0..49).map(move |x| datum + (f64::from(x) * 2.0 - 80.0) / 20.0));
    let bed = Bed::new([-80.0, -32.0], 2.0, [49, 33], heights.collect()).expect("plane beach");
    let map = Map::prepare(
        bed,
        vec![Body {
            name: "Sea".into(),
            kind: BodyKind::Sea,
            level: datum,
            stage: Stage {
                min: 0.0,
                max: 0.5,
                value: 0.0,
            },
            fetch: 100_000.0,
            gains: [1.0, 0.0, 0.0, 0.0],
        }],
        thrum_water_model::Profile {
            margin_height: 1.5,
            swash_ratio: 0.7,
        },
    )
    .expect("beach map");
    let long = [(0.4667, 1.0), (0.2333, -1.0)]
        .map(|(amplitude, side)| Component {
            wave_vector: [
                FRAC_1_SQRT_2 * TAU / 25.0,
                side * FRAC_1_SQRT_2 * TAU / 25.0,
            ],
            amplitude,
            phase: 0.0,
        })
        .to_vec();
    let field = Field {
        layers: std::array::from_fn(|i| {
            LongWaves::new(if i == 0 { long.clone() } else { Vec::new() }, CHOPPINESS)
                .expect("field")
        }),
    };
    let spectrum = Spectrum {
        long,
        cascades: [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("calm")),
        tail_slope_variance: 0.0,
    };
    (map, spectrum, field)
}

fn test_map(lake: bool, datum: f64) -> Map {
    let bottom: Vec<_> = (0..25)
        .map(|i| {
            datum
                + if lake && (i % 5 == 0 || i % 5 == 4 || i / 5 == 0 || i / 5 == 4) {
                    2.0
                } else {
                    -4.0
                }
        })
        .collect();
    Map::prepare(
        Bed::new([-12.0; 2], 6.0, [5, 5], bottom).expect("bounded bed"),
        vec![Body {
            name: if lake { "lake" } else { "sea" }.into(),
            kind: if lake {
                BodyKind::Basin { seed: [0.0; 2] }
            } else {
                BodyKind::Sea
            },
            level: datum,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 100.0,
            gains: if lake {
                [0.2, 0.0, 0.0, 0.0]
            } else {
                [1.0, 0.0, 0.0, 0.0]
            },
        }],
        thrum_water_model::Profile {
            margin_height: 1.5,
            swash_ratio: 0.7,
        },
    )
    .expect("model map")
}

/// A model fixture and its matching water contribution.
pub struct Case {
    /// One of the named lens fixtures.
    pub name: &'static str,
    map: Map,
    /// Spectrum shared by the model oracle and the contribution.
    pub spectrum: Spectrum,
    field: Field,
    column: [f64; 2],
    mapped: bool,
}

#[derive(Clone, Copy)]
enum ShortAt {
    Rest,
    FootprintGain,
    FootprintWeight,
}

impl Case {
    /// Prepare the named fixture at the given vertical datum.
    /// # Panics
    /// Panics if the fixed model inputs are invalid or the name is unknown.
    #[must_use]
    pub fn new(name: &'static str, datum: f64) -> Self {
        if name == "short-trough" {
            return Self::short_trough(true, datum);
        }
        if name == "mapped-trough" {
            let mut case = Self::short_trough(true, datum);
            case.name = name;
            case.column = [0.0; 2];
            let sea = case.map.body(0).expect("sea").clone();
            let heights = (0..25).map(|i| datum - 0.21 - f64::from(i % 5) * 0.01);
            let bed =
                Bed::new([-12.0; 2], 6.0, [5, 5], heights.collect()).expect("shallow sloping bed");
            case.map = Map::prepare(bed, vec![sea], case.map.profile()).expect("mapped trough");
            return case;
        }
        if name == "trade-admission" {
            let mut case = Self::new("sea", datum);
            case.name = name;
            case.column = [-293.0, 20.0];
            case.mapped = true;
            case.spectrum =
                Spectrum::build(&crate::island::Wind::Trade.forcing()).expect("trade spectrum");
            case.map
                .set_wave_gains(0, [1.0; 4])
                .expect("offshore wave gains");
            return case;
        }
        if name == "trough-wet" {
            let mut case = Self::new("mapped-trough", datum);
            case.name = name;
            return case;
        }
        if name == "impact" {
            let mut case = Self::new("lake", datum);
            case.name = name;
            case.spectrum.long.clear();
            return case;
        }
        if name == "carried-crest" {
            let mut case = Self::new("sea", datum);
            case.name = name;
            let k = TAU / 80.0;
            case.spectrum.long = vec![Component {
                wave_vector: [0.0, k],
                amplitude: 1.0,
                phase: TAU / 4.0 - 2.0 * k,
            }];
            case.spectrum.cascades[0] =
                Cascade::new(160.0, &[(0, 48, 0.125, -1.875 * 48.0 * TAU / 160.0)])
                    .expect("short crest at the long wave's rest place");
            case.field.layers[0] =
                LongWaves::new(case.spectrum.long.clone(), 2.0).expect("long horizontal throw");
            case.map
                .set_wave_gains(0, [1.0, 1.0, 0.0, 0.0])
                .expect("two geometric bands");
            return case;
        }
        if name == "carried-gain" {
            let mut case = Self::new("carried-crest", datum);
            case.name = name;
            case.mapped = true;
            let sea = case.map.body(0).expect("sea").clone();
            let heights = (-24..=24).flat_map(|z| {
                let depth = (0.08 + 1.45 * f64::from(z).max(0.0)).min(4.0);
                std::iter::repeat_n(datum - depth, 49)
            });
            let bed = Bed::new(
                [case.column[0] - 12.0, -12.0],
                0.5,
                [49, 49],
                heights.collect(),
            )
            .expect("sloping short-wave bed");
            case.map = Map::prepare(
                bed,
                vec![sea],
                thrum_water_model::Profile {
                    margin_height: 0.1,
                    swash_ratio: 0.1,
                },
            )
            .expect("mapped crest");
            return case;
        }
        if name == "carried-weight" {
            let mut case = Self::new("lip", datum);
            case.name = name;
            let phase = -case.column[0] * 12.0 * TAU / 160.0;
            case.spectrum.cascades[0] = Cascade::new(160.0, &[(12, 0, 0.15, phase)])
                .expect("short crest on the transformed shore");
            case.map
                .set_wave_gains(0, [1.0, 1.0, 0.0, 0.0])
                .expect("transformed short wave");
            return case;
        }
        let column = match name {
            "sea" => [81.0, 0.0],
            "trade" => [-293.0, 20.0],
            "lake" => [0.0, 0.0],
            "swash" => [2.5, 0.0],
            "lip" => [-28.755, 3.2295],
            _ => panic!("unknown water lens fixture {name}"),
        };
        let (mut map, mut spectrum, _) = beach(datum);
        let mapped = !matches!(name, "sea" | "trade");
        if matches!(name, "lake" | "sea" | "trade") {
            map = test_map(name == "lake", datum);
            if name == "trade" {
                spectrum =
                    Spectrum::build(&crate::island::Wind::Trade.forcing()).expect("Trade spectrum");
                map.set_wave_gains(0, [1.0; 4])
                    .expect("offshore wave gains");
            }
        } else {
            map.enable_surf(SurfProfile::default())
                .expect("shore transform");
            if name == "lip" {
                for wave in &mut spectrum.long {
                    wave.phase -= wave.frequency() * (STEP * 3).as_secs_f64();
                }
            }
        }
        let field = Field {
            layers: std::array::from_fn(|index| {
                LongWaves::new(
                    if index == 0 {
                        spectrum.long.clone()
                    } else {
                        Vec::new()
                    },
                    if name == "trade" {
                        crate::profile(&spectrum).waves.choppiness
                    } else {
                        CHOPPINESS
                    },
                )
                .expect("oracle field")
            }),
        };
        Self {
            name,
            map,
            spectrum,
            field,
            column,
            mapped,
        }
    }

    /// One short mode with a 0.1 m trough on the lens's centre column.
    /// # Panics
    /// Panics if the fixed mode or sea declaration is invalid.
    #[must_use]
    pub fn short_trough(mapped: bool, datum: f64) -> Self {
        let mut case = Self::new("sea", datum);
        case.name = "short-trough";
        case.mapped = mapped;
        case.spectrum.long.clear();
        case.spectrum.cascades[0] =
            Cascade::new(160.0, &[(0, 12, 0.005, std::f64::consts::PI)]).expect("short wave");
        case.field.layers = [
            Vec::new(),
            case.spectrum.cascades[0].components().to_vec(),
            Vec::new(),
            Vec::new(),
        ]
        .map(|components| LongWaves::new(components, 0.0).expect("short wave oracle"));
        case.map
            .set_wave_gains(0, [0.0, 1.0, 0.0, 0.0])
            .expect("offshore short wave gain");
        case
    }

    /// Build the same render contribution with or without lens composition.
    /// # Panics
    /// Panics if the fixed optical profile is invalid.
    #[must_use]
    pub fn water(&self, underwater: bool) -> Water {
        let mut profile = Profile {
            optics: Some(Optics {
                extinction: [if matches!(self.name, "mapped-trough" | "trough-wet") {
                    100.0
                } else {
                    10.0
                }; 3],
                downwelling: [0.0; 3],
                ..Optics::default()
            }),
            underwater,
            ..Profile::default()
        };
        profile.waves.choppiness = self.field.layers[0].choppiness();
        if self.name == "impact" {
            profile.interaction = Some(thrum_water::interaction::Limits::default());
        }
        let live = Arc::new(Mutex::new(self.map.clone()));
        if self.mapped {
            Water::mapped(live, self.spectrum.clone(), profile)
        } else {
            Water::new(live, 0, self.spectrum.clone(), profile)
        }
        .expect("lens surface")
    }

    /// The eye on the highest water surface in the center column.
    /// # Panics
    /// Panics if the center is dry or the folded lip has fewer than three crossings.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the camera uses GPU metres"
    )]
    pub fn eye(&self) -> [f32; 3] {
        if self.name == "impact" {
            return [
                0.0,
                (self.map.row(0, self.column).level + 0.07) as f32,
                -0.1,
            ];
        }
        if matches!(self.name, "short-trough" | "mapped-trough") {
            return [
                self.column[0] as f32,
                (self.map.row(0, self.column).level - 0.03) as f32,
                -0.1,
            ];
        }
        if self.name == "trade-admission" {
            let [x, z] = self.column;
            return [
                x as f32,
                (self.map.row(0, self.column).level + 2.0) as f32,
                (z - 0.1) as f32,
            ];
        }
        let triangles = self.triangles();
        let highest = highest_triangle(self.column, triangles.iter().copied())
            .expect("drawn sheet covers the lens centre");
        if self.name == "lip" {
            let mut heights: Vec<_> = triangles
                .into_iter()
                .filter_map(|triangle| highest_triangle(self.column, [triangle]))
                .collect();
            heights.sort_unstable_by(f64::total_cmp);
            heights.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
            assert!(
                heights.len() >= 3,
                "the lip must fold three drawn surfaces into the lens column: {heights:?}"
            );
        }
        let lens_height = if self.name == "trough-wet" {
            highest + f64::from(0.1 * (0.5 * FOV).tan()) - 0.02
        } else {
            highest
        };
        [
            self.column[0] as f32,
            lens_height as f32,
            self.column[1] as f32 - 0.1,
        ]
    }

    /// One Boolean per near-plane pixel from the CPU model, in image row order.
    /// # Panics
    /// Panics if the supplied view fails to repeat its world columns down the image.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "the test extent and pixel coordinates fit f32"
    )]
    pub fn expected(&self, view: &SyntheticView, [width, height]: [u32; 2]) -> Vec<bool> {
        let inverse = Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
        let point = |x: u32, y: u32| {
            let h = inverse
                * Vec4::new(
                    (x as f32 + 0.5).mul_add(2.0 / width as f32, -1.0),
                    (y as f32 + 0.5).mul_add(-2.0 / height as f32, 1.0),
                    1.0,
                    1.0,
                );
            h / h.w
        };
        // A level near plane repeats its world columns down the image. Solve
        // each column once, then compare to each pixel's own vertical point.
        let triangles = (NAMES.contains(&self.name)
            || matches!(
                self.name,
                "sea" | "carried-crest" | "carried-gain" | "carried-weight" | "trough-wet"
            ))
        .then(|| self.triangles());
        let columns: Vec<_> = (0..width)
            .map(|x| {
                let p = point(x, 0);
                let xz = [p.x, p.z];
                let point = xz.map(f64::from);
                let height = if let Some(faces) = &triangles {
                    highest_triangle(point, faces.iter().copied())
                } else {
                    let surface = self
                        .map
                        .surface(&self.field, point, 0.0)
                        .expect("per-column admission fixture oracle");
                    surface.wet.then_some(surface.sample.height)
                };
                (xz, height)
            })
            .collect();
        let mut expected = Vec::with_capacity(width as usize * height as usize);
        for y in 0..height {
            for (x, &(xz, height)) in (0..width).zip(&columns) {
                let p = point(x, y);
                assert_eq!(xz, [p.x, p.z]);
                expected.push(height.is_some_and(|surface| surface > f64::from(p.y)));
            }
        }
        expected
    }

    /// The old footprint bound for the synthetic carried-crest witness.
    /// Both trains run along z, and this fixture's level near plane has fixed z.
    /// # Panics
    /// Panics when called on another fixture.
    #[must_use]
    pub fn carried_crest_column_bound(&self) -> f64 {
        assert_eq!(self.name, "carried-crest");
        let long = self.field.layers[0]
            .forward(self.column, 0.0)
            .expect("long wave")
            .displacement[1];
        let band = &self.spectrum.cascades[0];
        let short = texels(self.column, band.length() / 256.0, |p| {
            band.direct(p, 0.0).displacement
        })[1];
        self.map.row(0, self.column).level + long + short
    }

    fn triangles(&self) -> Vec<[SheetPoint; 3]> {
        self.triangles_at(ShortAt::Rest)
    }

    /// Difference in centre-column heights if only the chosen short-wave term
    /// is evaluated on the lens footprint. Other terms retain their rest values.
    /// # Panics
    /// Panics when the case does not isolate a gain or weight.
    #[must_use]
    pub fn footprint_short_error(&self) -> f64 {
        let alternative = match self.name {
            "carried-gain" => ShortAt::FootprintGain,
            "carried-weight" => ShortAt::FootprintWeight,
            _ => panic!("expected a carried short-wave gain or weight fixture"),
        };
        let cut =
            |at| highest_triangle(self.column, self.triangles_at(at)).expect("covered lens column");
        cut(ShortAt::Rest) - cut(alternative)
    }

    fn triangles_at(&self, short_at: ShortAt) -> Vec<[SheetPoint; 3]> {
        let mut variances = self.field.variances();
        let short = self.spectrum.cascades.each_ref().map(Cascade::variance);
        let grids = self.spectrum.cascades.each_ref().map(|cascade| {
            (!cascade.components().is_empty()).then(|| cascade.inverse_transform(0.0))
        });
        variances[1..].copy_from_slice(&short);
        let surf = self
            .map
            .surf_sheet(&self.field, 0, 0.0)
            .ok()
            .map(|sheet| sheet.with_short_wave_variances(short));
        let footprint = self.map.sample(self.column).expect("lens footprint row");
        let footprint_gains = limit(
            footprint.row,
            footprint.depth,
            variances,
            self.map.profile(),
        )
        .gains;
        let footprint_weight = surf.as_ref().map_or(1.0, |sheet| {
            self.map.surf_short_wave_weight(
                self.column,
                footprint,
                variances,
                footprint.row.level + sheet.forward(self.column).height,
            )
        });
        // These witnesses are in the near grid, away from every ring seam.
        let spacing = if self.name == "lake" { 0.5 } else { 0.25 };
        let origin = self.column.map(f64::floor);
        let vertex = |rest| {
            let row = self.map.sample(rest).expect("lens witness has one body");
            let gains = limit(row.row, row.depth, variances, self.map.profile()).gains;
            let long = surf.as_ref().map_or_else(
                || {
                    let displacement = texels(rest, 1.0, |p| {
                        self.field.layers[0]
                            .forward(p, 0.0)
                            .expect("long wave")
                            .displacement
                    })
                    .map(|value| value * gains[0]);
                    SheetPoint {
                        world: [rest[0] + displacement[0], rest[1] + displacement[2]],
                        height: displacement[1],
                    }
                },
                |sheet| sheet.forward(rest),
            );
            let mut point = SheetPoint {
                height: row.row.level + long.height,
                ..long
            };
            let weight = if matches!(short_at, ShortAt::FootprintWeight) {
                footprint_weight
            } else {
                self.map
                    .surf_short_wave_weight(rest, row, variances, point.height)
            };
            for (band, cascade) in self.spectrum.cascades.iter().enumerate() {
                let local_gain = if matches!(short_at, ShortAt::FootprintGain) {
                    footprint_gains[band + 1]
                } else {
                    gains[band + 1]
                };
                if local_gain == 0.0 || short[band] == 0.0 {
                    continue;
                }
                let length = cascade.length();
                let blend = ((spacing - length / 128.0) / (3.0 * length / 128.0)).clamp(0.0, 1.0);
                let gain = weight * local_gain * (1.0 - blend * blend * (3.0 - 2.0 * blend));
                let step = length / 256.0;
                let grid = grids[band].as_ref().expect("active cascade");
                let [x, y, z] = texels(rest, step, |p| {
                    let [x, z] = p.map(|v| periodic_texel(v, step));
                    grid[z * GRID_SIZE + x].displacement
                });
                point.world[0] += gain * x;
                point.world[1] += gain * z;
                point.height += gain * y;
            }
            point
        };
        let reach = surf.as_ref().map_or(0.0, |sheet| sheet.reach(self.column));
        let radius = if self.name == "lake" {
            8
        } else if reach > 4.0 {
            40
        } else {
            20
        };
        let stride = 2 * radius + 1;
        let vertices: Vec<_> = (-radius..=radius)
            .flat_map(|z| (-radius..=radius).map(move |x| [x, z]))
            .map(|offset| {
                vertex(std::array::from_fn(|axis| {
                    f64::from(offset[axis]).mul_add(spacing, origin[axis])
                }))
            })
            .collect();
        let stride = usize::try_from(stride).expect("bounded witness grid");
        (0..stride - 1)
            .flat_map(|z| {
                let vertices = &vertices;
                let surf = surf.is_some();
                (0..stride - 1).flat_map(move |x| {
                    let a = z * stride + x;
                    let [b, c, d] = [a + 1, a + stride, a + stride + 1];
                    if surf {
                        [[a, c, d], [a, d, b]]
                    } else {
                        [[a, c, b], [b, c, d]]
                    }
                    .map(|triangle| triangle.map(|i| vertices[i]))
                })
            })
            .collect()
    }

    /// Model expectation in the browser proof's translated frame.
    /// # Panics
    /// Panics if the fixed camera or model fails the authored fixture rules.
    #[must_use]
    pub fn browser_expected(&self) -> Vec<bool> {
        let eye = self.eye();
        let view = SyntheticView::perspective(
            eye,
            target(eye),
            [0.0, 1.0, 0.0],
            FOV.to_degrees().to_radians(),
            16.0 / 9.0,
            0.1,
            30_000.0,
        )
        .expect("level camera");
        self.expected(&view, EXTENT)
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "indices wrap into the fixed periodic grid"
)]
fn periodic_texel(point: f64, step: f64) -> usize {
    (point / step - 0.5).round().rem_euclid(256.0) as usize
}

fn texels(point: [f64; 2], step: f64, sample: impl Fn([f64; 2]) -> [f64; 3]) -> [f64; 3] {
    let grid = point.map(|value| value / step - 0.5);
    let base = grid.map(f64::floor);
    let fraction: [f64; 2] = std::array::from_fn(|axis| grid[axis] - base[axis]);
    let mut sum = [0.0; 3];
    for offset in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]] {
        let tap = std::array::from_fn(|axis| (base[axis] + offset[axis] + 0.5) * step);
        let weight =
            (1.0 - (fraction[0] - offset[0]).abs()) * (1.0 - (fraction[1] - offset[1]).abs());
        for (value, component) in sum.iter_mut().zip(sample(tap)) {
            *value += weight * component;
        }
    }
    sum
}

fn target([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, y, z + 1.0]
}

/// The browser test entry point, using the ordinary sky shell and cooked opaque pass.
/// # Errors
/// Returns an error for invalid controls, cooked assets or clock setup.
/// # Panics
/// Panics if the authored impact fixture lacks its interaction capability.
pub fn browser_game(case: &Case, underwater: bool, dry: bool) -> Result<SkyGame, Box<dyn Error>> {
    let mut eye = case.eye();
    if matches!(case.name, "short-trough" | "mapped-trough") {
        if !dry {
            eye[2] += 160.0 / 24.0;
        }
    } else if case.name == "trade-admission" {
        if !dry {
            eye[1] -= 4.0;
        }
    } else if dry {
        eye[1] += 3.0;
    }
    let camera = Camera::look_at(eye, target(eye), FOV.to_degrees());
    let game = SkyGame::new(Controls::new(), camera, |_, sky| {
        sky.sun_elevation_degrees = 75.0;
    })
    .with_ground(None)
    .with_tick_duration(STEP)?;
    let panel = MeshPlacement::new("meshes/lens-panel", "textures/white").at([
        eye[0],
        eye[1],
        eye[2] + 0.101,
    ]);
    let shader = r"
        fn cooked_surface(input: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
            var out = input;
            out.base_colour = vec4(0.0, 0.0, 0.0, 1.0);
            out.emissive = vec3(10000.0);
            return out;
        }
    ";
    let water = case.water(underwater);
    let game = if case.name == "impact" {
        let wake = water.interaction_mailbox().expect("lake interaction");
        game.with_fixed_update(move |tick, _| {
            if tick == STONE.tick {
                wake.post_impulse(STONE);
            }
        })
    } else {
        game
    };
    Ok(game
        .with_meshes_and_material(vec![panel], Some(("lens witness", shader)))?
        .with_contribution(Box::new(water), || ContentStatus::Drawn(1)))
}
