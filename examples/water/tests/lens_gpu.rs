//! Model-side columns and submitted Metal water composition for a split lens.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
#[path = "lens_gpu/review.rs"]
mod review;

#[cfg(target_os = "macos")]
mod metal {
    use crate::support::optics::{Panel, Rig, attach, camera, sun};
    use std::sync::{Mutex, MutexGuard, PoisonError};
    use thrum_render::runtime::FrameSample;
    use thrum_test_support::Gpu;
    use water_scene::lens_probe::{Case, EXTENT, FOV};

    static DEVICE: Mutex<()> = Mutex::new(());

    pub(super) fn gpu(test: &str) -> Option<(MutexGuard<'static, ()>, Gpu)> {
        // Each fixture can retain several Metal timestamp query sets at once.
        let serial = DEVICE.lock().unwrap_or_else(PoisonError::into_inner);
        thrum_test_support::gpu(test, Some(wgpu::Backend::Metal)).map(|gpu| (serial, gpu))
    }

    pub(super) fn runs_gather(sample: &FrameSample) -> bool {
        sample
            .gpu_pass_ns
            .as_ref()
            .expect("Metal timing")
            .passes()
            .iter()
            .any(|pass| pass.name.contains("water gather"))
    }

    pub(super) fn check_dry_panel(dry: &mut Rig, plain: &mut Rig, context: &str) {
        let (sample, exposure, colour) = dry.scene_colour_with_sample();
        let (baseline, baseline_exposure, reference) = plain.scene_colour_with_sample();
        assert_eq!(sample.observation, baseline.observation);
        assert_eq!(sample.tick, baseline.tick);
        assert_eq!(
            sample.presentation_alpha.to_bits(),
            baseline.presentation_alpha.to_bits()
        );
        let cut = sample
            .gpu_pass_ns
            .as_ref()
            .expect("Metal timing")
            .passes()
            .iter()
            .any(|pass| pass.name.contains("water lens cut"));
        assert!(
            !runs_gather(&sample) && !cut,
            "{context}: dry lens scheduled underwater work"
        );
        assert!(
            !runs_gather(&baseline),
            "{context}: ordinary optics ran a gather"
        );
        assert_eq!(exposure.to_bits(), baseline_exposure.to_bits());

        // An opaque emissive panel immediately behind the near plane isolates
        // the lens. Distant ordinary-wave lighting varies across Metal captures.
        let expected_floor = 12_000.0 * exposure;
        for (pixel, (dry, ordinary)) in colour.iter().zip(&reference).enumerate() {
            assert!(
                ordinary[0] > expected_floor,
                "{context}: panel obscured at pixel {pixel}: {ordinary:?}"
            );
            assert_eq!(
                dry.map(f32::to_bits),
                ordinary.map(f32::to_bits),
                "{context}: dry lens altered the panel at pixel {pixel}: {dry:?} vs {ordinary:?}"
            );
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::print_stderr,
        reason = "bounded lens fixture and its proof log"
    )]
    pub(super) fn check(case: &Case, gpu: &Gpu) {
        let (extent, panel_gaps): (_, &[f32]) =
            if std::env::var_os("THRUM_WATER_LENS_1440P").is_some() {
                (
                    [2560, 1440],
                    &[
                        0.000_000_3,
                        0.000_000_2,
                        0.000_000_55,
                        0.000_001,
                        0.000_000_1,
                        0.000_000_4,
                    ],
                )
            } else {
                (EXTENT, &[0.000_001])
            };
        let [width, height] = extent;
        let eye = case.eye();
        if case.name == "carried-crest" {
            let lowest = f64::from(eye[1] - 0.1 * (FOV * 0.5).tan());
            assert!(
                lowest > case.carried_crest_column_bound() + 0.05,
                "the whole lens must stand above the footprint bound"
            );
        }
        let view = camera(eye, [eye[0], eye[1], eye[2] + 1.0], FOV, 1.0 / 32768.0);
        let capture = |gap, underwater| {
            let panel = Panel {
                centre: [eye[0], eye[1], (eye[2] + 0.11) + gap],
                half_size: [0.2, 0.16, 0.01],
                radiant: true,
            };
            let mut rig = Rig::with_extent(gpu, &view, sun(0.0, 0.0), &[panel], None, extent);
            attach(&mut rig, case.water(underwater));
            rig.host.set_timing_requested(underwater);
            let (sample, _, colour) = rig.scene_colour_with_sample();
            if underwater {
                assert!(
                    (sample.tick == 1 && sample.presentation_alpha.to_bits() == 0)
                        || (sample.tick == 0
                            && sample.presentation_alpha.to_bits() == 1.0_f32.to_bits()),
                    "{} model needs time zero",
                    case.name
                );
                let timings = sample.gpu_pass_ns.expect("Metal lens cut timings");
                assert!(
                    timings
                        .passes()
                        .iter()
                        .any(|pass| pass.name.contains("water lens cut")),
                    "{} did not cut the GPU mesh",
                    case.name
                );
            }
            colour
        };
        if case.name == "trade" {
            let eye = [eye[0], 100.0, eye[2]];
            let sky_view = camera(eye, [eye[0], eye[1], eye[2] + 1.0], FOV, 1.0 / 32768.0);
            let mut dry = Rig::with_extent(gpu, &sky_view, sun(3.0, 0.0), &[], None, EXTENT);
            attach(&mut dry, case.water(true));
            dry.host.set_timing_requested(true);
            let timings = dry.frame().gpu_pass_ns.expect("Metal dry-lens timings");
            assert!(
                !timings
                    .passes()
                    .iter()
                    .any(|pass| pass.name.contains("water lens cut")
                        || pass.name.contains("water through lens"))
            );
        }
        // At 1440p the near clip and the folded sheet occasionally cover the
        // probe. Choose the most visible ordinary panel at each pixel, then
        // read the corresponding underwater arm at that same panel position.
        // The lens pose and the model's expected pixels stay fixed.
        let mut panels = panel_gaps
            .iter()
            .map(|&gap| (capture(gap, false), capture(gap, true)));
        let (mut ordinary, mut under) = panels.next().expect("near-plane panel");
        for (reference, candidate) in panels {
            for (index, colour) in reference.into_iter().enumerate() {
                if colour[0] > ordinary[index][0] {
                    ordinary[index] = colour;
                    under[index] = candidate[index];
                }
            }
        }
        let expected = case.expected(&view, extent);
        let is_under = |x, y| expected[(y * width + x) as usize];
        let fraction = expected.iter().filter(|&&wet| wet).count() as f32 / expected.len() as f32;
        let split_range = if case.name == "trough-wet" {
            0.1..0.4
        } else {
            0.25..0.75
        };
        assert!(split_range.contains(&fraction), "{}: {fraction}", case.name);
        let mut wrong = Vec::new();
        let mut missing = 0;
        let mut first_missing = None;
        for y in 2..height - 2 {
            for x in 1..width - 1 {
                let wet = is_under(x, y);
                if [
                    is_under(x - 1, y),
                    is_under(x + 1, y),
                    is_under(x, y - 2),
                    is_under(x, y + 2),
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
            "Metal {} at {width}×{height}: {fraction:.3} of the lens is under, {} disagreements beyond two pixels, {missing} without a readable panel",
            case.name,
            wrong.len()
        );
        assert!(
            wrong.is_empty() && missing == 0,
            "{}: disagreements {:?}, first obscured panel pixel {first_missing:?}",
            case.name,
            &wrong[..wrong.len().min(8)]
        );
    }

    pub(super) fn check_meniscus(gpu: &Gpu) {
        let mut case = Case::new("sea", 0.0);
        case.spectrum.long.clear();
        let fov = FOV;
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

        // At a fixed pose the centre texel samples both sides of the middle
        // of the meniscus as the raster jitter moves its near-plane point.
        rig.host.continuous_source_mut().view =
            view(-0.5 * pixel_height).with_jitter_phase(0, EXTENT);
        let jittered: [f32; 4] = std::array::from_fn(|_| rig.scene_colour().1[middle][0]);
        let low = jittered.into_iter().fold(f32::INFINITY, f32::min);
        let high = jittered.into_iter().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            high - low > 0.30 * (dry - wet),
            "meniscus ignored the jittered raster position: {jittered:?}"
        );
    }
}

thrum_test_support::macos_gpu_test! {
fn metal_meniscus_grades_the_first_submerged_pixel() {
    let Some((_serial, gpu)) = metal::gpu("subpixel waterline") else { return };
    metal::check_meniscus(&gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_sea_lens_reads_the_near_plane_and_skips_the_dry_gather() {
    let Some((_serial, gpu)) = metal::gpu("split sea lens") else { return };
    metal::check(&water_scene::lens_probe::Case::new("trade", 0.0), &gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_lake_lens_reads_the_near_plane() {
    let Some((_serial, gpu)) = metal::gpu("split lake lens") else { return };
    metal::check(&water_scene::lens_probe::Case::new("lake", 0.0), &gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_swash_lens_reads_the_near_plane() {
    let Some((_serial, gpu)) = metal::gpu("split swash lens") else { return };
    metal::check(&water_scene::lens_probe::Case::new("swash", 0.0), &gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_lip_lens_reads_the_highest_of_three_surfaces() {
    let Some((_serial, gpu)) = metal::gpu("split lens below the lip") else { return };
    metal::check(&water_scene::lens_probe::Case::new("lip", 0.0), &gpu);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_admits_the_short_crest_carried_in_from_outside_the_footprint() {
    let Some((_serial, gpu)) = metal::gpu("off-footprint carried crest") else { return };
    metal::check(&water_scene::lens_probe::Case::new("carried-crest", 0.0), &gpu);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_uses_the_short_wave_gain_and_weight_at_the_reachable_rest_place() {
    use water_scene::lens_probe::{Case, EXTENT, FOV};
    let Some((_serial, gpu)) = metal::gpu("carried short-wave gain and weight") else { return };
    let pixel_height = 0.2 * f64::from((FOV * 0.5).tan()) / f64::from(EXTENT[1]);
    for name in ["carried-gain", "carried-weight"] {
        let case = Case::new(name, 0.0);
        let error = case.footprint_short_error();
        eprintln!("{name}: evaluating that term on the footprint shifts the line {:.2} rows", error / pixel_height);
        assert!(error.abs() > 2.0 * pixel_height, "{name}: {error:.6} m");
        metal::check(&case, &gpu);
    }
}
}

thrum_test_support::macos_gpu_test! {
fn metal_composes_a_lens_two_centimetres_under_the_short_trough() {
    let Some((_serial, gpu)) = metal::gpu("two centimetres under a short trough") else { return };
    metal::check(&water_scene::lens_probe::Case::new("trough-wet", 0.0), &gpu);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_skips_a_dry_short_wave_trough_and_admits_the_crest() {
    use crate::support::optics::{Panel, Rig, attach, camera, sun};
    use water_scene::lens_probe::{Case, EXTENT, FOV};
    let Some((_serial, gpu)) = metal::gpu("dry short-wave trough") else { return };
    for case in [
        Case::short_trough(false, 0.0),
        Case::short_trough(true, 0.0),
        Case::new("mapped-trough", 0.0),
    ] {
        let [x, y, trough] = case.eye();
        let look = |z| camera([x, y, z], [x, y, z + 1.0], FOV, 1.0 / 32768.0);
        let view = look(trough);
        assert!(case.expected(&view, EXTENT).into_iter().all(|wet| !wet));
        let panel = Panel {
            centre: [x, y, (trough + 0.11) + 0.000_001],
            half_size: [0.2, 0.16, 0.01],
            radiant: true,
        };
        let mut dry = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[panel], None, EXTENT);
        attach(&mut dry, case.water(true));
        let mut plain = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[panel], None, EXTENT);
        attach(&mut plain, case.water(false));
        dry.host.set_timing_requested(true);
        plain.host.set_timing_requested(true);
        metal::check_dry_panel(&mut dry, &mut plain, case.name);
        if case.name == "mapped-trough" {
            let low = camera([x, y - 0.02, trough], [x, y - 0.02, trough + 1.0], FOV, 1.0 / 32768.0);
            let expected = case.expected(&low, EXTENT);
            assert!(expected.contains(&true) && expected.contains(&false));
            dry.host.continuous_source_mut().view = low;
            assert!(metal::runs_gather(&dry.frame()), "using the unrestricted trough would skip the wet edge");
        }
        let crest = look(160.0 / 24.0 - 0.1);
        assert!(case.expected(&crest, EXTENT).into_iter().all(|wet| wet));
        dry.host.continuous_source_mut().view = crest;
        assert!(metal::runs_gather(&dry.frame()), "the same lens at the short-wave crest needs the gather");
    }
}
}

thrum_test_support::macos_gpu_test! {
fn metal_skips_dry_trade_water_as_the_footprint_moves() {
    use crate::support::optics::{Panel, Rig, attach, camera, sun};
    use water_scene::lens_probe::{Case, EXTENT, FOV};
    let Some((_serial, gpu)) = metal::gpu("dry Trade sea") else { return };
    let case = Case::new("trade-admission", 0.0);
    let [x, y, z] = case.eye();
    let look = |dx, y| camera([x + dx, y, z], [x + dx, y, z + 1.0], FOV, 1.0 / 32768.0);
    let panel = Panel {
        centre: [x + 4.5, y, (z + 0.11) + 0.000_001],
        half_size: [5.0, 0.16, 0.01],
        radiant: true,
    };
    let mut lens = Rig::with_extent(&gpu, &look(0.0, y), sun(0.0, 0.0), &[panel], None, EXTENT);
    attach(&mut lens, case.water(true));
    let mut plain = Rig::with_extent(&gpu, &look(0.0, y), sun(0.0, 0.0), &[panel], None, EXTENT);
    attach(&mut plain, case.water(false));
    lens.host.set_timing_requested(true);
    plain.host.set_timing_requested(true);
    for dx in [0.0, 0.08, 0.16, 1.0, 9.02, 0.0] {
        lens.host.continuous_source_mut().view = look(dx, y);
        plain.host.continuous_source_mut().view = look(dx, y);
        metal::check_dry_panel(&mut lens, &mut plain, &format!("dry Trade lens at {dx} m"));
    }
    lens.host.continuous_source_mut().view = look(0.0, y - 4.0);
    assert!(metal::runs_gather(&lens.frame()), "submerged Trade lens omitted its gather");
}
}

thrum_test_support::macos_gpu_test! {
fn metal_admits_a_dry_lens_over_a_live_wake_window() {
    use crate::support::{Fixture, optics::water_configured};
    use std::time::Duration;
    use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
    use thrum_render::runtime::FrameSample;
    use thrum_water::{Optics, Profile, interaction::Limits};
    use thrum_water_model::Impulse;
    use water_scene::lens_probe::{EXTENT, FOV};

    let Some((_serial, gpu)) = metal::gpu("wake lens admission") else { return };
    let view = |x| Camera::look_at([x, 5.0, -0.1], [x, 5.0, 0.9], FOV.to_degrees());
    let water = water_configured(None, Profile {
        optics: Some(Optics::default()),
        underwater: true,
        interaction: Some(Limits::default()),
        ..Profile::default()
    });
    let mailbox = water.interaction_mailbox().expect("prepared interaction capability");
    let report = water.report();
    let mut controls = Controls::new();
    let camera_x = controls.slider("camera.x", "Camera x", 0.0..=300.0)
        .default(0.0).add().expect("camera x control");
    let game = SkyGame::new(controls, view(0.0), move |controls, sky| {
        sky.camera = view(controls.get(camera_x));
    })
    .with_tick_duration(Duration::from_millis(20)).expect("fixed clock")
    .with_contribution(Box::new(water), || ContentStatus::Drawn(1));
    let mut rig = Fixture::from_game(&gpu, EXTENT, game, report);
    let gathers = |frame: &FrameSample| frame.gpu_pass_ns.as_ref().expect("Metal timing")
        .passes().iter().any(|pass| pass.name.contains("water gather"));
    let mut elapsed = 0.0;
    let mut submit = |rig: &mut Fixture| {
        let frame = rig.frame(elapsed);
        elapsed += 0.021;
        frame
    };
    let still = (0..12).map(|_| submit(&mut rig))
        .find(|frame| frame.gpu_pass_ns.is_some())
        .unwrap_or_else(|| panic!("settled scene with Metal timestamps: {:?}", rig.host.timing_stats()));
    assert!(!gathers(&still), "a dry lens over the reset wake window should be idle");
    let tick = still.tick + 1;
    mailbox.post_impulse(Impulse { id: 1, tick, place: [0.0; 2], vertical: -100.0, radius: 0.4 });
    let active = submit(&mut rig);
    assert!(active.tick >= tick, "the impact tick must submit");
    assert!(gathers(&active), "a live wake window admits the lens even five metres above water");
    assert!(active.gpu_pass_ns.as_ref().expect("active Metal timing").passes()
        .iter().any(|pass| pass.name.contains("water lens cut")));
    assert!(gathers(&submit(&mut rig)), "the window remains live after the impact tick");
    rig.parameters(r#"{"camera.x":300}"#);
    assert!(!gathers(&submit(&mut rig)), "a dry lens clear of the wake window should be idle");
}
}

thrum_test_support::macos_gpu_test! {
fn metal_composes_a_lens_below_a_lake_impact_ring() {
    use crate::support::optics::{Panel, Rig, attach, camera, sun};
    use thrum_water::interaction::Witness;
    use water_scene::lens_probe::{Case, EXTENT, FOV, STONE};
    let Some((_serial, gpu)) = metal::gpu("lake impact lens split") else { return };
    let case = Case::new("impact", 0.0);
    let eye = case.eye();
    let view = camera(eye, [eye[0], eye[1], eye[2] + 1.0], FOV, 1.0 / 32768.0);
    let panel = Panel { centre: [0.0, eye[1], 0.01001], half_size: [0.2, 0.16, 0.01], radiant: true };
    let arm = |underwater| {
        let mut water = case.water(underwater);
        let wake = water.tap_interaction();
        let impulses = water.interaction_mailbox().expect("lake interaction");
        let mut rig = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[panel], None, EXTENT);
        *rig.host.world_mut() = thrum_app::sky::StillWorld::with_tick_duration(water_scene::lens_probe::STEP).expect("impact clock");
        attach(&mut rig, water);
        let still = rig.scene_colour_with_sample();
        assert!(still.0.tick < STONE.tick, "first witness precedes the stone");
        impulses.post_impulse(STONE);
        (still.2, rig, wake)
    };
    let (plain_still, mut plain, _) = arm(false);
    let (lens_still, mut lens, wake) = arm(true);
    assert_eq!(plain_still, lens_still, "the calm level leaves the whole near plane dry");
    lens.host.set_timing_requested(true);
    let mut first_wet = None;
    let mut peak = f32::NEG_INFINITY;
    for step in 0..64 {
        let (control, _, ordinary) = plain.scene_colour_with_sample();
        let (sample, _, composed) = lens.scene_colour_with_sample();
        assert_eq!(sample.tick, control.tick);
        assert_eq!(sample.presentation_alpha.to_bits(), control.presentation_alpha.to_bits());
        let witness = wake.latest().expect("submitted impact field");
        let endpoint = witness.endpoints[1];
        let centre = |level: usize| {
            let spacing = Witness::CELL_METRES[level];
            let [x, z] = endpoint.origins[level].map(|origin| (-origin / spacing).round());
            let cell = |value: f32| (0..Witness::SIDE).zip(0_u16..)
                .find(|(_, i)| f32::from(*i).to_bits() == value.to_bits()).expect("witness cell").0;
            let [x, z] = [x, z].map(cell);
            let offset = ((endpoint.offsets[level] + z * Witness::SIDE + x) * 8) as u64;
            let raw = thrum_test_support::readback::buffer(&lens.device, &lens.queue, &witness.buffer, offset..offset + 4);
            f32::from_le_bytes(raw.try_into().unwrap())
        };
        peak = peak.max(centre(0) + centre(1));
        let pixels = ordinary.iter().zip(composed).filter(|(a, b)| a[0] > 1.0 && b[0] < 0.7 * a[0]).count();
        if pixels > 16 {
            let timings = sample.gpu_pass_ns.expect("Metal impact timing");
            assert!(timings.passes().iter().any(|pass| pass.name.contains("water lens cut")));
            first_wet = Some((step, sample.tick, pixels));
            break;
        }
    }
    let (step, tick, pixels) = first_wet.unwrap_or_else(|| panic!("the incoming stone ring never covered the dry lens, peak submitted height {peak}"));
    eprintln!("Metal lake impact composed {pixels} pixels at tick {tick}, step {step}, peak submitted height {peak} m");
}
}
