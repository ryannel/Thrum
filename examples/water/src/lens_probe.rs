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
use thrum_sea_model::{Cascade, Component, LongWaves, Spectrum};
use thrum_water::{Optics, Profile, Water};
use thrum_water_model::{Bed, Body, BodyKind, CutConfig, Field, Map, Stage, SurfProfile, cut};

/// Simulation step shared by the wave probes.
pub const STEP: Duration = Duration::from_nanos(16_666_667);
/// Long-wave deformation used by the water and its model.
pub const CHOPPINESS: f64 = 0.5;
/// Vertical field of view of the level probe in radians.
pub const FOV: f32 = 0.85;
/// Vertical translation that puts the browser fixtures within the sky shell's camera range.
pub const BROWSER_DATUM: f64 = 8.0;
/// The four model fixtures submitted by the lens tests.
pub const NAMES: [&str; 4] = ["sea", "lake", "swash", "lip"];
/// Submitted grid used by the matching checks.
pub const EXTENT: [u32; 2] = [48, 27];

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

impl Case {
    /// Prepare the named fixture at the given vertical datum.
    /// # Panics
    /// Panics if the fixed model inputs are invalid or the name is unknown.
    #[must_use]
    pub fn new(name: &'static str, datum: f64) -> Self {
        let column = match name {
            "sea" => [81.0, 0.0],
            "lake" => [0.0, 0.0],
            "swash" => [2.5, 0.0],
            "lip" => [-28.755, 3.2295],
            _ => panic!("unknown water lens fixture {name}"),
        };
        let (mut map, mut spectrum, _) = beach(datum);
        let mapped = name != "sea";
        if name == "lake" || name == "sea" {
            map = test_map(name == "lake", datum);
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
                    CHOPPINESS,
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

    /// Build the same render contribution with or without lens composition.
    /// # Panics
    /// Panics if the fixed optical profile is invalid.
    #[must_use]
    pub fn water(&self, underwater: bool) -> Water {
        let mut profile = Profile {
            optics: Some(Optics {
                extinction: [10.0; 3],
                downwelling: [0.0; 3],
                ..Optics::default()
            }),
            underwater,
            ..Profile::default()
        };
        profile.waves.choppiness = CHOPPINESS;
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
        let highest = self
            .map
            .surface(&self.field, self.column, 0.0)
            .expect("model at the lens centre");
        assert!(highest.wet);
        if self.name == "lip" {
            let sheet = self
                .map
                .surf_sheet(&self.field, 0, 0.0)
                .expect("two crossing trains");
            assert!(
                cut(&sheet, self.column, CutConfig::default())
                    .expect("fold")
                    .hits
                    >= 3
            );
        }
        [
            self.column[0] as f32,
            highest.sample.height as f32,
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
        let columns: Vec<_> = (0..width)
            .map(|x| {
                let p = point(x, 0);
                let xz = [p.x, p.z];
                let answer = self
                    .map
                    .surface(&self.field, xz.map(f64::from), 0.0)
                    .expect("per-column lens oracle");
                (xz, answer)
            })
            .collect();
        let mut expected = Vec::with_capacity(width as usize * height as usize);
        for y in 0..height {
            for (x, &(xz, answer)) in (0..width).zip(&columns) {
                let p = point(x, y);
                assert_eq!(xz, [p.x, p.z]);
                expected.push(answer.wet && answer.sample.height > f64::from(p.y));
            }
        }
        expected
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

fn target([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, y, z + 1.0]
}

/// The browser test entry point, using the ordinary sky shell and cooked opaque pass.
/// # Errors
/// Returns an error for invalid controls, cooked assets or clock setup.
pub fn browser_game(case: &Case, underwater: bool, dry: bool) -> Result<SkyGame, Box<dyn Error>> {
    let mut eye = case.eye();
    if dry {
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
    Ok(game
        .with_meshes_and_material(vec![panel], Some(("lens witness", shader)))?
        .with_contribution(Box::new(case.water(underwater)), || ContentStatus::Drawn(1)))
}
