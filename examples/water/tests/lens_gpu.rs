//! Model-side columns and submitted Metal water composition for a split lens.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
mod metal {
    use crate::support::optics::{Panel, Rig, attach, camera, sun};
    use water_scene::lens_probe::{Case, EXTENT, FOV};

    #[expect(
        clippy::cast_precision_loss,
        clippy::print_stderr,
        reason = "bounded lens fixture and its proof log"
    )]
    pub(super) fn check(case: &Case, gpu: &thrum_test_support::Gpu) {
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
        let view = camera(eye, [eye[0], eye[1], eye[2] + 1.0], FOV, 1.0 / 32768.0);
        let capture = |gap, underwater| {
            let panel = Panel {
                centre: [eye[0], eye[1], (eye[2] + 0.11) + gap],
                half_size: [0.2, 0.16, 0.01],
                radiant: true,
            };
            let mut rig = Rig::with_extent(gpu, &view, sun(0.0, 0.0), &[panel], None, extent);
            attach(&mut rig, case.water(underwater));
            rig.scene_colour().1
        };
        if case.name == "sea" {
            let eye = [eye[0], 3.0, eye[2]];
            let sky_view = camera(eye, [eye[0], eye[1], eye[2] + 1.0], FOV, 1.0 / 32768.0);
            let mut dry = Rig::with_extent(gpu, &sky_view, sun(3.0, 0.0), &[], None, EXTENT);
            attach(&mut dry, case.water(true));
            dry.host.set_timing_requested(true);
            let timings = dry.frame().gpu_pass_ns.expect("Metal dry-lens timings");
            assert!(
                !timings
                    .passes()
                    .iter()
                    .any(|pass| pass.name.contains("water gather"))
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
        assert!(
            (0.25..0.75).contains(&fraction),
            "{}: {fraction}",
            case.name
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

    pub(super) fn check_meniscus(gpu: &thrum_test_support::Gpu) {
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
    let Some(gpu) = thrum_test_support::gpu("subpixel waterline", Some(wgpu::Backend::Metal)) else { return };
    metal::check_meniscus(&gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_sea_lens_reads_the_near_plane_and_skips_the_dry_gather() {
    let Some(gpu) = thrum_test_support::gpu("split sea lens", Some(wgpu::Backend::Metal)) else { return };
    metal::check(&water_scene::lens_probe::Case::new("sea", 0.0), &gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_lake_lens_reads_the_near_plane() {
    let Some(gpu) = thrum_test_support::gpu("split lake lens", Some(wgpu::Backend::Metal)) else { return };
    metal::check(&water_scene::lens_probe::Case::new("lake", 0.0), &gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_swash_lens_reads_the_near_plane() {
    let Some(gpu) = thrum_test_support::gpu("split swash lens", Some(wgpu::Backend::Metal)) else { return };
    metal::check(&water_scene::lens_probe::Case::new("swash", 0.0), &gpu);
}
}
thrum_test_support::macos_gpu_test! {
fn metal_lip_lens_reads_the_highest_of_three_surfaces() {
    let Some(gpu) = thrum_test_support::gpu("split lens below the lip", Some(wgpu::Backend::Metal)) else { return };
    metal::check(&water_scene::lens_probe::Case::new("lip", 0.0), &gpu);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_skips_a_dry_short_wave_trough_and_admits_the_crest() {
    use crate::support::optics::{Rig, attach, camera, sun};
    use water_scene::lens_probe::{Case, EXTENT, FOV};
    let Some(gpu) = thrum_test_support::gpu("dry short-wave trough", Some(wgpu::Backend::Metal)) else { return };
    for mapped in [false, true] {
        let case = Case::short_trough(mapped, 0.0);
        let [x, y, trough] = case.eye();
        let look = |z| camera([x, y, z], [x, y, z + 1.0], FOV, 1.0 / 32768.0);
        let view = look(trough);
        let mut dry = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[], None, EXTENT);
        attach(&mut dry, case.water(true));
        let mut plain = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[], None, EXTENT);
        attach(&mut plain, case.water(false));
        assert!(-0.03_f64 - 0.1 * (f64::from(FOV) * 0.5).tan() > -0.08);
        let centre = case.spectrum.cascades[0].direct([81.0, 0.0], 0.0).displacement[1];
        assert!(centre < -0.099);
        let submitted = dry.scene_colour().1;
        let reference = plain.scene_colour().1;
        let equal = submitted.iter().zip(&reference)
            .filter(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
            .count();
        assert_eq!(equal, (EXTENT[0] * EXTENT[1]) as usize, "dry pixels remain visually dry");
        dry.host.set_timing_requested(true);
        let gathers = |rig: &mut Rig| rig.frame().gpu_pass_ns.expect("Metal timing")
            .passes().iter().any(|pass| pass.name.contains("water gather"));
        assert!(!gathers(&mut dry), "the dry lens scheduled the underwater gather");
        dry.host.continuous_source_mut().view = look(160.0 / 24.0 - 0.1);
        assert!(gathers(&mut dry), "the same lens at the short-wave crest needs the gather");
    }
}
}

thrum_test_support::macos_gpu_test! {
fn metal_skips_dry_trade_water_as_the_footprint_moves() {
    use crate::support::optics::{Rig, attach, camera, sun};
    use water_scene::lens_probe::{Case, EXTENT, FOV};
    let Some(gpu) = thrum_test_support::gpu("dry Trade sea", Some(wgpu::Backend::Metal)) else { return };
    let case = Case::new("trade-admission", 0.0);
    let [x, y, z] = case.eye();
    let look = |dx, y| camera([x + dx, y, z], [x + dx, y, z + 1.0], FOV, 1.0 / 32768.0);
    let mut lens = Rig::with_extent(&gpu, &look(0.0, y), sun(0.0, 0.0), &[], None, EXTENT);
    attach(&mut lens, case.water(true));
    let mut plain = Rig::with_extent(&gpu, &look(0.0, y), sun(0.0, 0.0), &[], None, EXTENT);
    attach(&mut plain, case.water(false));
    lens.host.set_timing_requested(true);
    let gathers = |rig: &mut Rig| rig.frame().gpu_pass_ns.expect("Metal timing")
        .passes().iter().any(|pass| pass.name.contains("water gather"));
    for dx in [0.0, 0.08, 0.16, 1.0, 9.02, 0.0] {
        lens.host.continuous_source_mut().view = look(dx, y);
        plain.host.continuous_source_mut().view = look(dx, y);
        assert!(!gathers(&mut lens), "dry Trade lens at {dx} m scheduled the underwater gather");
        let colour = lens.scene_colour().1;
        let reference = plain.scene_colour().1;
        assert!(colour.iter().zip(&reference).all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits)),
            "the dry Trade lens changed output at {dx} m");
    }
    lens.host.continuous_source_mut().view = look(0.0, y - 4.0);
    assert!(gathers(&mut lens), "submerged Trade lens omitted its gather");
}
}
