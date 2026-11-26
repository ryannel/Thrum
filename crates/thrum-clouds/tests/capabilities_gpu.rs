//! The six Weather + Shade combinations, with a real resident placed volume.
#[cfg(target_os = "macos")]
#[expect(
    dead_code,
    reason = "shared placed fixture has tests with other entry points"
)]
mod placed_fixture;
#[cfg(target_os = "macos")]
#[expect(
    dead_code,
    reason = "shared scene fixture retains the no-overlay entry point"
)]
mod scene_fixture;

thrum_test_support::macos_gpu_test! {
    fn metal_creates_and_runs_only_declared_cloud_capabilities() {
        run();
    }
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::print_stdout,
    reason = "retain the named Metal capability inventory"
)]
fn run() {
    use std::{sync::Arc, time::Duration};
    use thrum_clouds::{Shade, SkyProfile};
    use thrum_render::{contribution::SceneContributions, runtime::FrameConfig};

    const SHADE: [&str; 6] = [
        "cloud weather map",
        "cloud Sun path near",
        "cloud Sun path middle",
        "cloud Sun path far",
        "cloud ground field",
        "cloud far transmittance",
    ];
    const SKY: [&str; 5] = [
        "cloud hemisphere light",
        "cloud trace",
        "cloud resolve",
        "cloud compose",
        "cloud internal field",
    ];
    const GROUND: [&str; 4] = [
        "cloud weather map",
        "cloud Sun path",
        "cloud ground field",
        "cloud far transmittance",
    ];

    let Some(gpu) = thrum_test_support::open(
        "cloud capability inventory",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    assert_eq!(gpu.gpu.adapter.get_info().backend, wgpu::Backend::Metal);
    let fixture = placed_fixture::Fixture::new();
    let angle = 20.0_f32.to_radians();
    let sun = [angle.cos(), angle.sin(), 0.0];
    let view = placed_fixture::column_view(0.0);
    let extent = [66; 2];
    pollster::block_on(async {
        let mut samples = [Vec::new(), Vec::new()];
        let mut prior = std::collections::BTreeMap::new();
        for shapes in [false, true] {
            for (sky, world_light) in [(false, false), (true, false), (true, true)] {
                let mut host = scene_fixture::prepare_view_with_overlay(
                    &gpu.capabilities,
                    &gpu.queue,
                    Some(placed_fixture::GROUND),
                    FrameConfig {
                        render_extent: extent,
                        surface: Some(thrum_render::present::SurfaceProfile {
                            format: wgpu::TextureFormat::Bgra8Unorm,
                            extent,
                        }),
                    },
                    sun,
                    1,
                    &view,
                    (false, world_light),
                )
                .await;
                let assets = fixture.assets();
                let lease = assets.lease();
                let shade = if shapes {
                    fixture.shade(lease.clone(), [25.0, 40.0], sky)
                } else {
                    let shade = Shade::new(fixture.config.clone()).expect("Weather and Shade");
                    if sky {
                        shade
                            .with_sky(SkyProfile {
                                extent,
                                step: 5.0,
                                light_samples: 0,
                                refresh_stride: 1,
                            })
                            .expect("Sky")
                    } else {
                        shade
                    }
                };
                let input = shade.input();
                input
                    .publish(Arc::clone(&fixture.state))
                    .expect("publish the same weather");
                let mut members: Vec<Box<dyn thrum_render::contribution::SceneContribution>> =
                    Vec::new();
                if shapes {
                    members.push(Box::new(assets));
                }
                members.push(Box::new(shade));
                host.set_scene_contribution(Box::new(SceneContributions::new(members)))
                    .expect("cloud capabilities");
                let output = placed_fixture::output(&gpu);
                let mut now = Duration::ZERO;
                thrum_test_support::wait::until(
                    "placed volume lease",
                    Duration::from_secs(4),
                    || {
                        placed_fixture::frame(&gpu, &mut host, &output, now);
                        now += Duration::from_millis(17);
                        (!shapes || lease.offered().is_some()).then_some(())
                    },
                );
                placed_fixture::frame(&gpu, &mut host, &output, now);

                let graph = host.scene_graph().expect("compiled scene");
                let names: Vec<_> = graph
                    .pass_names()
                    .filter_map(|name| {
                        name.split_once("cloud ")
                            .map(|(_, suffix)| format!("cloud {suffix}"))
                    })
                    .collect();
                let mut expected: Vec<_> =
                    SHADE
                        .into_iter()
                        .chain(SKY.into_iter().filter(|name| {
                            sky && (!world_light || *name != "cloud hemisphere light")
                        }))
                        .chain(world_light.then_some("cloud sky overlay"))
                        .chain(shapes.then_some("cloud placed weather map"))
                        .map(str::to_owned)
                        .collect();
                expected.sort();
                let mut actual = names.clone();
                actual.sort();
                assert_eq!(actual, expected, "scene's full cloud schedule");
                // These hosts share a device generation and its ledger.
                // Count only creations made while this arm owned the host.
                let current = host.creation_report().named_pipelines;
                let creations: std::collections::BTreeMap<_, _> = current
                    .iter()
                    .filter_map(|(name, total)| {
                        let created = total - prior.get(name).copied().unwrap_or(0);
                        (created != 0).then(|| (name.clone(), created))
                    })
                    .collect();
                prior = current;
                let shade_builds = creations["cloud weather map"];
                assert!(shade_builds > 0);
                for name in GROUND {
                    assert_eq!(creations.get(name), Some(&shade_builds), "{name}");
                }
                let count = |name| creations.get(name).copied().unwrap_or(0);
                let check_creation = |name, enabled| {
                    let value = count(name);
                    assert_eq!(value > 0, enabled, "{name} created {value} times");
                };
                for name in SKY {
                    check_creation(name, sky);
                }
                for name in [
                    "cloud sky overlay",
                    "atmosphere overlay diffuse irradiance",
                    "atmosphere overlay specular prefilter",
                ] {
                    check_creation(name, world_light);
                }
                for name in [
                    "atmosphere diffuse irradiance",
                    "atmosphere sky specular prefilter",
                ] {
                    check_creation(name, !world_light);
                }
                check_creation("cloud placed weather map", shapes);
                let resources = input.resource_bytes();
                let has = |needle: &str| {
                    resources
                        .iter()
                        .any(|(name, size)| name.contains(needle) && *size > 0)
                };
                assert_eq!(has("layer texture"), sky);
                assert_eq!(has("detail noise"), sky);
                assert_eq!(has("sky overlay"), world_light);
                assert_eq!(has("overlay rows"), world_light);
                assert_eq!(has("placed weather map"), shapes);
                assert_eq!(has("placed weather windows"), shapes);
                assert_eq!(
                    input.shapes_report().map(|report| report.volume_instances),
                    shapes.then_some(1)
                );
                assert_eq!(input.sky_products().is_some(), sky);
                let ground = placed_fixture::ground(&gpu, &input);
                let previous = &mut samples[usize::from(shapes)];
                if previous.is_empty() {
                    previous.clone_from(&ground);
                }
                assert_eq!(
                    *previous, ground,
                    "ground field depends on Sky or World light"
                );
                if world_light && !shapes {
                    check_held_overlay(&gpu, &mut host, &output, extent, now);
                }
                println!(
                    "Metal: Sky {sky}, World light {world_light}, Shapes {shapes}\n  passes {names:?}\n  creations {creations:?}\n  resources {resources:?}"
                );
            }
        }
        assert_ne!(
            samples[0], samples[1],
            "placed volume casts shade within the compared field"
        );
        assert!(samples[1].iter().any(|&byte| byte != 0));
    });
}

#[cfg(target_os = "macos")]
fn check_held_overlay(
    gpu: &thrum_test_support::Opened,
    host: &mut thrum_render::runtime::FrameHost<scene_fixture::World, scene_fixture::Source>,
    output: &wgpu::TextureView,
    extent: [u32; 2],
    now: std::time::Duration,
) {
    use std::time::Duration;
    use thrum_render::runtime::SurfaceOutput;

    let sample = |host: &mut thrum_render::runtime::FrameHost<_, _>, elapsed| {
        let report = host
            .drive(
                elapsed,
                Some(&SurfaceOutput {
                    view: output,
                    extent,
                }),
            )
            .expect("submit overlay cadence frame");
        assert!(report.submitted);
        let submission = host.last_submission().expect("cadence submission");
        thrum_test_support::wait::until("cadence completion", Duration::from_secs(2), || {
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("cadence GPU poll");
            while let Some(frame) = host.poll_completed().expect("cadence completion") {
                if frame.submission_order == submission {
                    return Some(frame);
                }
            }
            None
        })
    };
    for i in 1..=9 {
        sample(host, now + Duration::from_millis(i * 17));
    }
    let quiet = sample(host, now + Duration::from_millis(170));
    let passes = |frame: thrum_render::runtime::FrameSample| {
        frame
            .gpu_pass_ns
            .expect("Metal timestamps")
            .passes()
            .iter()
            .map(|pass| pass.name.to_string())
            .collect::<Vec<_>>()
    };
    let quiet = passes(quiet);
    host.continuous_source_mut().view = placed_fixture::column_view(8.0);
    let moved = passes(sample(host, now + Duration::from_millis(187)));
    for name in [
        "cloud sky overlay",
        "atmosphere diffuse irradiance",
        "atmosphere reflection mip 0",
        "atmosphere reflection mip 6",
    ] {
        assert!(
            !quiet.iter().any(|pass| pass.ends_with(name)),
            "settled frame ran {name}"
        );
        assert!(
            moved.iter().any(|pass| pass.ends_with(name)),
            "moved frame omitted {name}: {moved:?}"
        );
    }
}
