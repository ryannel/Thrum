//! Submitted-depth controls for refined river surfaces and the shared rain array.
mod optical_sources;

#[cfg(target_os = "macos")]
use crate::support::{
    optics::{Panel, Rig, attach, camera, sun},
    river_store,
};
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex};
#[cfg(target_os = "macos")]
use thrum_render::synthetic::{DisplayOverlay, SyntheticView};
#[cfg(target_os = "macos")]
use thrum_sea_model::{Cascade, Spectrum};
#[cfg(target_os = "macos")]
use thrum_water::{Optics, Profile, Water};
#[cfg(target_os = "macos")]
use thrum_water_model::{
    Bed, Body, BodyKind, Map, OpenSky, RainBands, RainBandsProfile, RateHistory, Stage,
};

#[cfg(target_os = "macos")]
fn depth_view(view: &SyntheticView) -> SyntheticView {
    view.with_temporal_inputs(true)
        .with_display_overlay(DisplayOverlay::TemporalInputs)
}

#[cfg(target_os = "macos")]
fn rain_sea(rain_rate: Option<f64>, amplitude: f64, underwater: bool) -> Water {
    let mut map = Map::prepare(
        Bed::new([-10.0; 2], 10.0, [3, 3], vec![-20.0; 9]).expect("deep sea"),
        vec![Body {
            name: "Sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 10_000.0,
            gains: [1.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 1.0,
            swash_ratio: 0.7,
        },
    )
    .expect("bounded map");
    if let Some(rate) = rain_rate {
        let mut history = RateHistory::dry(1.0 / 60.0, -1, 100);
        history.rates.fill(rate / 3600.0);
        let profile = RainBandsProfile {
            count: 256.0,
            ..Default::default()
        };
        let table = thrum_rain_model::RingProfile {
            count: profile.count,
            ..Default::default()
        }
        .table();
        map.set_rain(Some(
            RainBands::new(profile, Arc::new(table), history, Arc::new(OpenSky))
                .expect("shared rain array"),
        ));
    }
    let mut cascades =
        [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("empty cascade"));
    if amplitude > 0.0 {
        cascades[0] = Cascade::new(160.0, &[(1, 0, amplitude, 0.0)]).expect("one wind mode");
    }
    Water::mapped(
        Arc::new(Mutex::new(map)),
        Spectrum {
            long: vec![],
            cascades,
            tail_slope_variance: 0.0,
        },
        Profile {
            underwater,
            optics: underwater.then(Optics::default),
            ..Default::default()
        },
    )
    .expect("rain sea")
}

thrum_test_support::macos_gpu_test! {
fn metal_keeps_wind_displacement_in_the_shared_rain_array() {
    let Some((_serial, gpu)) = super::metal::gpu("wind displacement with zero rainfall") else {
        return;
    };
    let extent = [65, 37];
    let centre = ((extent[1] / 2) * extent[0] + extent[0] / 2) as usize;
    let view = depth_view(&camera([0.0, 2.0, -0.001], [0.0; 3], 0.3, 1.0 / 32768.0));
    let inverse = glam::Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
    let height = |rain, amplitude| {
        let mut rig = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[], None, extent);
        attach(&mut rig, rain_sea(rain, amplitude, false));
        rig.frame();
        inverse
            .project_point3(glam::vec3(0.0, 0.0, rig.submitted_depth()[centre]))
            .y
    };
    let ordinary = height(None, 0.25) - height(None, 0.0);
    let shared = height(Some(0.0), 0.25) - height(Some(0.0), 0.0);
    eprintln!(
        "Submitted wind lift is {ordinary:.6} m without rain and {shared:.6} m with zero rainfall"
    );
    assert!(ordinary > 0.6 && (shared - ordinary).abs() < 0.002);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_admits_rain_rings_that_reach_a_dry_still_water_lens() {
    check_rain_ring_lens(50.0, true);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_keeps_the_lens_dry_above_still_water_without_rain() {
    check_rain_ring_lens(0.0, false);
}
}

fn check_rain_ring_lens(rate: f64, active: bool) {
    let Some((_serial, gpu)) = super::metal::gpu("rain ring lens admission") else {
        return;
    };
    let extent = [65, 37];
    let height = 0.000_002_5;
    let (x, z) = (3.5, 0.5);
    let view = depth_view(
        &SyntheticView::perspective(
            [x, 0.1 + height, z],
            [x, -1.0, z],
            [0.0, 0.0, 1.0],
            1.55,
            16.0 / 9.0,
            0.1,
            30_000.0,
        )
        .expect("downward camera"),
    );
    let mut rig = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[], None, extent);
    let water = rain_sea(Some(rate), 0.0, true);
    let report = water.report();
    attach(&mut rig, water);
    rig.frame();
    let clipped = rig
        .submitted_depth()
        .iter()
        .filter(|&&depth| depth == 0.0)
        .count();
    let admitted = report.lock().expect("committed water").underwater_admitted;
    assert_eq!(
        admitted,
        Some(active),
        "lens admission at {rate} mm/h and {height:.7} m above still water"
    );
    assert_eq!(
        clipped > 0,
        active,
        "{clipped} clipped pixels at {rate} mm/h and {height:.7} m above still water"
    );
}

// Each station submits three independent render hosts. Keep the station
// witnesses separate so the repository gate can time them individually.
thrum_test_support::macos_gpu_test! {
fn metal_lens_cuts_the_solved_river_depth_near_x338() {
    check_river_depth([337.83, 5.23]);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_lens_cuts_the_solved_river_depth_near_x330() {
    check_river_depth([330.1, 5.2]);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_lens_cuts_the_solved_river_depth_near_x325() {
    check_river_depth([325.4, 5.0]);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_lens_cuts_the_solved_river_depth_near_x312() {
    check_river_depth([312.0, 5.0]);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_lens_cuts_the_solved_river_depth_near_x295() {
    check_river_depth([295.0, 5.0]);
}
}

thrum_test_support::macos_gpu_test! {
fn metal_lens_cuts_the_solved_river_depth_near_x286() {
    check_river_depth([285.5, 5.0]);
}
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::cast_possible_truncation,
    clippy::print_stderr,
    reason = "bounded GPU stations and submitted centre rows in the proof log"
)]
fn check_river_depth([x, z]: [f64; 2]) {
    use water_scene::{channel::forcing, island::Wind, river};
    let Some((_serial, gpu)) = super::metal::gpu("refined river lens depth") else {
        return;
    };
    let mut forcing = forcing(Wind::Trade);
    forcing.wind_direction = 0.42;
    let (map, _) = river::map(&forcing, Some(&river_store())).expect("shared solved rivers");
    let spectrum = Spectrum::build(&forcing).expect("wind and swell");
    let extent = [97, 111];
    let [width, height] = extent;
    let water = |underwater| {
        let mut profile = water_scene::profile(&spectrum);
        profile.optics = Some(Optics {
            extinction: [10.0; 3],
            downwelling: [0.0; 3],
            ..Default::default()
        });
        profile.underwater = underwater;
        Water::mapped(Arc::new(Mutex::new(map.clone())), spectrum.clone(), profile)
            .expect("mapped river")
    };
    let site = map.sample([x, z]).expect("wet station");
    assert!(matches!(
        map.body(site.body).expect("owner").kind,
        BodyKind::Chart(_)
    ));
    let (x, z, level) = (x as f32, z as f32, site.row.level as f32);
    let view = depth_view(&camera(
        [x, level + 0.5, z - 0.000_01],
        [x, level, z],
        0.5,
        1.0 / 32768.0,
    ));
    let inverse = glam::Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
    let mut rig = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[], None, extent);
    attach(&mut rig, water(false));
    let sampled = rig.frame();
    let depth = rig.submitted_depth()[(height / 2 * width + width / 2) as usize];
    assert!(depth > 0.0, "drawn river at {x},{z}");
    let [x, y, z] = inverse
        .project_point3(glam::vec3(0.0, 0.0, depth))
        .to_array();
    drop(rig);

    let view = camera([x, y, z - 0.1], [x, y, z + 1.0], 0.85, 1.0 / 32768.0);
    let panel = Panel {
        centre: [x, y, z + 0.010_002],
        half_size: [0.2, 0.16, 0.01],
        radiant: true,
    };
    let arm = |underwater| {
        let mut rig = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[panel], None, extent);
        attach(&mut rig, water(underwater));
        rig.scene_colour_with_sample()
    };
    let (plain, _, ordinary) = arm(false);
    let (composed, _, image) = arm(true);
    for frame in [&sampled, &plain] {
        assert_eq!(frame.tick, composed.tick);
        assert_eq!(
            frame.presentation_alpha.to_bits(),
            composed.presentation_alpha.to_bits()
        );
    }
    let centre = height / 2;
    let mut first_wet = None;
    let mut mismatches = 0;
    for row in 0..height {
        let pixel = (row * width + width / 2) as usize;
        assert!(ordinary[pixel][0] > 0.2, "panel at {x}, {z}, row {row}");
        let wet = image[pixel][0] < 0.7 * ordinary[pixel][0];
        if wet {
            first_wet.get_or_insert(row);
        }
        if row.abs_diff(centre) > 2 && wet != (row > centre) {
            mismatches += 1;
        }
    }
    eprintln!(
        "river {}, station {x:.4},{z:.4}: drawn y {y:.6}, expected row {centre}, first wet {first_wet:?}, disagreements outside two rows {mismatches}",
        site.body
    );
    assert_eq!(mismatches, 0, "lens and submitted river depth at {x}, {z}");
}
