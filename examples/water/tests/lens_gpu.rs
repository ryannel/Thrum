//! Model-side columns and submitted Metal water composition for a split lens.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
mod metal {
    use glam::{Mat4, Vec4};
    use std::sync::{Arc, Mutex};
    use thrum_sea_model::{LongWaves, Spectrum};
    use thrum_water::{Optics, Profile, Water};
    use thrum_water_model::{Bed, Body, BodyKind, CutConfig, Field, Map, Stage, SurfProfile, cut};

    use crate::support::{
        optics::{Panel, Rig, attach, camera, sun},
        surf::{CHOPPINESS, STEP, beach},
    };

    const EXTENT: [u32; 2] = [48, 27];

    fn test_map(lake: bool) -> Map {
        let bottom: Vec<_> = (0..25)
            .map(|i| {
                if lake && (i % 5 == 0 || i % 5 == 4 || i / 5 == 0 || i / 5 == 4) {
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
                level: 0.0,
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

    pub(super) struct Case {
        name: &'static str,
        map: Map,
        spectrum: Spectrum,
        field: Field,
        column: [f64; 2],
        mapped: bool,
    }

    impl Case {
        pub(super) fn new(name: &'static str, column: [f64; 2]) -> Self {
            let (mut map, mut spectrum, _) = beach();
            let mapped = name != "sea";
            if name == "lake" || name == "sea" {
                map = test_map(name == "lake");
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

        fn water(&self, underwater: bool) -> Water {
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

        #[expect(
            clippy::cast_possible_truncation,
            clippy::print_stderr,
            reason = "test fixture world and small extent are in f32"
        )]
        pub(super) fn check(&self, gpu: &thrum_test_support::Gpu) {
            let extent = if std::env::var_os("THRUM_WATER_LENS_1440P").is_some() {
                [2560, 1440]
            } else {
                EXTENT
            };
            let [width, height] = extent;
            let highest = self
                .map
                .surface(&self.field, self.column, 0.0)
                .expect("model at the lens centre");
            assert!(highest.wet, "{}", self.name);
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
            let eye = [
                self.column[0] as f32,
                highest.sample.height as f32,
                self.column[1] as f32 - 0.1,
            ];
            let view = camera(eye, [eye[0], eye[1], eye[2] + 1.0], 0.85, 1.0 / 32768.0);
            let panel = Panel {
                centre: [eye[0], eye[1], eye[2] + 0.111],
                half_size: [0.2, 0.16, 0.01],
                radiant: true,
            };
            let capture = |underwater| {
                let mut rig = Rig::with_extent(gpu, &view, sun(0.0, 0.0), &[panel], None, extent);
                attach(&mut rig, self.water(underwater));
                rig.scene_colour().1
            };
            if self.name == "sea" {
                let eye = [eye[0], 3.0, eye[2]];
                let sky_view = camera(eye, [eye[0], eye[1], eye[2] + 1.0], 0.85, 1.0 / 32768.0);
                let mut dry = Rig::with_extent(gpu, &sky_view, sun(3.0, 0.0), &[], None, EXTENT);
                attach(&mut dry, self.water(true));
                dry.host.set_timing_requested(true);
                let timings = dry.frame().gpu_pass_ns.expect("Metal dry-lens timings");
                assert!(
                    !timings
                        .passes()
                        .iter()
                        .any(|pass| pass.name.contains("water gather"))
                );
            }
            let ordinary = capture(false);
            let under = capture(true);
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
            // This level camera repeats the same world columns from top to
            // bottom. Cache only those exact model queries and test each
            // pixel against its own height on the near plane.
            let columns: Vec<_> = (0..width)
                .map(|x| {
                    let p = point(x, 0);
                    let xz = [p.x, p.z];
                    let answer = self
                        .map
                        .surface(&self.field, xz.map(f64::from), 0.0)
                        .expect("per-column CPU oracle");
                    (xz, answer)
                })
                .collect();
            let mut expected = Vec::new();
            for y in 0..height {
                for x in 0..width {
                    let p = point(x, y);
                    let (xz, answer) = columns[x as usize];
                    assert_eq!(xz, [p.x, p.z]);
                    expected.push(answer.wet && answer.sample.height > f64::from(p.y));
                }
            }
            let is_under = |x, y| expected[(y * width + x) as usize];
            let fraction =
                expected.iter().filter(|&&wet| wet).count() as f32 / expected.len() as f32;
            assert!(
                (0.25..0.75).contains(&fraction),
                "{}: {fraction}",
                self.name
            );
            let mut wrong = Vec::new();
            let mut missing = 0;
            let mut first_missing = None;
            for y in 1..height - 1 {
                for x in 1..width - 1 {
                    let wet = is_under(x, y);
                    if [
                        is_under(x - 1, y),
                        is_under(x + 1, y),
                        is_under(x, y - 1),
                        is_under(x, y + 1),
                    ]
                    .contains(&!wet)
                    {
                        continue;
                    }
                    let i = (y * width + x) as usize;
                    if ordinary[i][0] <= 0.2 {
                        missing += 1;
                        first_missing.get_or_insert((x, y, ordinary[i]));
                        continue;
                    }
                    let measured = under[i][0] < ordinary[i][0] * 0.7;
                    if measured != wet {
                        wrong.push((x, y, ordinary[i][0], under[i][0]));
                    }
                }
            }
            eprintln!(
                "Metal {} at {width}×{height}: {fraction:.3} of the lens is under, {} disagreements beyond one pixel, {missing} without a readable panel",
                self.name,
                wrong.len()
            );
            assert!(
                wrong.is_empty() && missing == 0,
                "{}: disagreements {:?}, first obscured panel pixel {first_missing:?}",
                self.name,
                &wrong[..wrong.len().min(8)]
            );
        }
    }

    pub(super) fn check_meniscus(gpu: &thrum_test_support::Gpu) {
        let mut case = Case::new("sea", [81.0, 0.0]);
        case.spectrum.long.clear();
        let fov = 0.85_f32;
        let pixel_height = 0.2 * (fov * 0.5).tan() / EXTENT[1] as f32;
        let view = |y| camera([81.0, y, -0.1], [81.0, y, 0.9], fov, 1.0 / 32768.0);
        let panel = Panel {
            centre: [81.0, 0.0, 0.011],
            half_size: [0.2, 0.16, 0.01],
            radiant: true,
        };
        let mut rig = Rig::with_extent(
            gpu,
            &view(pixel_height),
            sun(0.0, 0.0),
            &[panel],
            None,
            EXTENT,
        );
        attach(&mut rig, case.water(true));
        let middle = ((EXTENT[1] / 2) * EXTENT[0] + EXTENT[0] / 2) as usize;
        let dry = rig.scene_colour().1[middle][0];
        rig.host.continuous_source_mut().view = view(-0.5 * pixel_height);
        let edge = rig.scene_colour().1[middle][0];
        rig.host.continuous_source_mut().view = view(-2.0 * pixel_height);
        let wet = rig.scene_colour().1[middle][0];
        let fraction = (edge - wet) / (dry - wet);
        assert!(dry - wet > 0.4, "panel separates the two compositions");
        assert!(
            (0.15..0.85).contains(&fraction),
            "subpixel meniscus did not grade the waterline: dry {dry}, edge {edge}, wet {wet}"
        );
    }
}

thrum_test_support::macos_gpu_test! {
fn metal_meniscus_grades_the_first_submerged_pixel() {
    let Some(gpu) = thrum_test_support::gpu("subpixel waterline", Some(wgpu::Backend::Metal)) else { return };
    metal::check_meniscus(&gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_sea_lens_reads_the_near_plane_and_skips_the_dry_gather() {
    let Some(gpu) = thrum_test_support::gpu("split sea lens", Some(wgpu::Backend::Metal)) else { return };
    metal::Case::new("sea", [81.0, 0.0]).check(&gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_lake_lens_reads_the_near_plane() {
    let Some(gpu) = thrum_test_support::gpu("split lake lens", Some(wgpu::Backend::Metal)) else { return };
    metal::Case::new("lake", [0.0, 0.0]).check(&gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_swash_lens_reads_the_near_plane() {
    let Some(gpu) = thrum_test_support::gpu("split swash lens", Some(wgpu::Backend::Metal)) else { return };
    metal::Case::new("swash", [2.5, 0.0]).check(&gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_lip_lens_reads_the_highest_of_three_surfaces() {
    let Some(gpu) = thrum_test_support::gpu("split lens below the lip", Some(wgpu::Backend::Metal)) else { return };
    metal::Case::new("lip", [-28.755, 3.2295]).check(&gpu);
}
}
