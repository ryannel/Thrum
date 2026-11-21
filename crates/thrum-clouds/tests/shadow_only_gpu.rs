//! Shade is independent of Sky under real detail-changing Sun rays.
mod shadow_scene;
use thrum_cloud_model::{Density, Field, Parameters, Profile, Sample, State, Type};
use thrum_clouds::{Shade, ShadeConfig, SkyProfile, SunLevel, SunProfile, WeatherProfile};
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{SceneContribution, SceneContributions},
    runtime::{FrameConfig, FrameError},
};

fn shade_config() -> ShadeConfig {
    let model = Profile {
        pitch: 1000.0,
        turret_pitch: 260.0,
        turret_least: 0.18,
        turret_fade: 0.12,
        period: 1800.0,
        detail_mean: 0.5,
        erosion: 0.4,
    };
    let row = Type {
        base: 100.0,
        depth: [100.0, 200.0],
        liquid_water: 0.0001,
        droplet_radius: 0.00001,
        deck: 0.3,
        profile: [0.2, 0.3],
    };
    let types = vec![
        row,
        Type {
            base: 120.0,
            depth: [150.0, 260.0],
            deck: 0.0,
            ..row
        },
    ];
    ShadeConfig {
        seed: 7,
        model,
        types,
        weather: WeatherProfile {
            size: 32,
            texels: vec![20.0, 80.0, 320.0, 1600.0],
            maximum_age: 0.5,
        },
        sun: SunProfile {
            size: 32,
            levels: [80.0, 320.0, 1600.0].map(|texel| SunLevel {
                texel,
                slices: 16,
                maximum_age: 0.5,
            }),
            ground_size: 64,
            lowest_elevation: 2.0_f64.to_radians(),
            border_fraction: 0.1,
        },
    }
}

#[test]
fn shade_device_limit_keeps_its_error_kind_in_a_contribution_list() {
    use thrum_render::capabilities::{
        DeviceProfile, DeviceRecipe, DeviceRestriction, OpenedDevice,
    };

    let Some(gpu) = thrum_test_support::gpu("grouped Shade GPU limits", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let OpenedDevice {
        queue,
        capabilities: caps,
        ..
    } = pollster::block_on(
        DeviceRecipe {
            profile: DeviceProfile::Portable,
            restriction: Some(&DeviceRestriction {
                limits: Some(wgpu::Limits {
                    max_storage_textures_per_shader_stage: 2,
                    ..wgpu::Limits::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
        .open_admitted(&gpu.instance, &gpu.adapter),
    )
    .expect("restricted Metal device");
    let config = shade_config();
    let mut host = pollster::block_on(shadow_scene::prepare(
        &caps,
        &queue,
        config.sun.ground_size,
        FrameConfig {
            render_extent: [64, 64],
            surface: None,
        },
    ));
    let shade = || Box::new(Shade::new(config.clone()).expect("valid Shade settings"));
    let direct: Box<dyn SceneContribution> = shade();
    let grouped = Box::new(SceneContributions::new(vec![
        Box::new(SceneContributions::default()),
        shade(),
    ]));
    for (route, contribution) in [("direct", direct), ("grouped", grouped)] {
        let error = host
            .attach_scene_contribution(contribution)
            .expect_err("device cannot prepare Shade");
        assert!(
            matches!(&error, FrameError::Gpu(reason) if reason.contains("weather map texture limits")),
            "{route}: {error:?}"
        );
    }
}

#[test]
fn both_recipes_keep_coarse_sun_paths_and_shadow_only_declares_no_layer() {
    let config = shade_config();
    let field =
        Field::new(config.seed, config.model, config.types.clone()).expect("authored test weather");
    let state = State::new(
        Parameters {
            coverage: 0.7,
            kind: 0.4,
            wind: [0.0; 2],
            time_scale: 1.0,
        },
        5.0,
        0,
    )
    .expect("static state");
    let mut maximum = 0.0_f64;
    let mut squared = 0.0;
    let mut changed = 0u32;
    let mut count = 0u32;
    for elevation in [20.0_f64, 60.0] {
        let angle = elevation.to_radians();
        let direction = [angle.cos(), angle.sin(), 0.0];
        for x in -4..4 {
            for z in -4..4 {
                let ray = Sample {
                    tick: 0,
                    origin: [f64::from(x) * 350.0, 100.0, f64::from(z) * 350.0],
                    direction,
                    length: 280.0 / direction[1],
                    steps: 256,
                };
                let coarse = field.optical_depth(&ray, &state, Density::Coarse, |_| None, 0.0);
                let full = field.optical_depth(&ray, &state, Density::Full, |_| None, 0.0);
                let error = ((-coarse).exp() - (-full).exp()).abs();
                maximum = maximum.max(error);
                squared += error * error;
                changed += u32::from(error > 1e-5);
                count += 1;
            }
        }
    }
    assert!(
        changed > 8,
        "fixture must expose extinction changed by detail: {changed}"
    );
    let rms = (squared / f64::from(count)).sqrt();
    eprintln!(
        "coarse/full Sun transmittance: probes={count}, changed={changed}, maximum={maximum}, RMS={rms}; design proof full/coarse 0.06–0.08 RMS inside cloud, local correction 0.02"
    );
    pollster::block_on(async {
        for (profile, recipe) in [
            (thrum_test_support::Profile::Portable, "portable"),
            (thrum_test_support::Profile::Native, "apple-tile"),
        ] {
            let Some(thrum_test_support::Opened {
                gpu, device, queue, ..
            }) = thrum_test_support::open(
                "shadow-only cloud capability",
                Some(wgpu::Backend::Metal),
                profile,
                &thrum_test_support::Restriction::default(),
            )
            else {
                return;
            };
            let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1)
                .with_recipe_override(recipe)
                .expect("recipe admitted");
            eprintln!(
                "recipe {recipe}: sampled textures per stage {}, storage textures {}, storage buffers {}",
                caps.limits().max_sampled_textures_per_shader_stage,
                caps.limits().max_storage_textures_per_shader_stage,
                caps.limits().max_storage_buffers_per_shader_stage
            );
            let mut products = Vec::new();
            for visible in [false, true] {
                let shade = Shade::new(config.clone()).expect("shade");
                let input = shade.input();
                input
                    .publish(std::sync::Arc::new(state.clone()))
                    .expect("weather lease");
                let shade = if visible {
                    shade
                        .with_sky(SkyProfile {
                            extent: [64, 64],
                            step: 10.0,
                            light_samples: 3,
                            refresh_stride: 2,
                        })
                        .expect("sky")
                } else {
                    shade
                };
                let host = shadow_scene::host(&device, &caps, &queue, shade, 64).await;
                let graph = host.scene_graph().expect("compiled scene graph");
                let names = graph.pass_names().collect::<Vec<_>>();
                if visible {
                    assert!(input.sky_products().is_some());
                    assert!(names.iter().any(|n| n.contains("cloud trace")));
                    assert!(
                        input
                            .resource_bytes()
                            .iter()
                            .any(|(name, _)| name == "cloud detail noise")
                    );
                } else {
                    assert!(!names.iter().any(|n| n.contains("cloud trace")
                        || n.contains("cloud resolve")
                        || n.contains("cloud compose")
                        || n.contains("hemisphere")));
                    assert!(input.sky_products().is_none());
                    let resources = input.resource_bytes();
                    assert!(
                        resources
                            .iter()
                            .any(|(name, _)| name == "cloud shape noise")
                    );
                    assert!(resources.iter().all(|(name, _)| !name.contains("layer")
                        && !name.contains("detail noise")
                        && !name.contains("phase")));
                }
                products.push(shadow_scene::ground(&device, &queue, &input));
            }
            assert_eq!(
                products[0], products[1],
                "Sky must not change any ground byte"
            );
        }
    });
}
