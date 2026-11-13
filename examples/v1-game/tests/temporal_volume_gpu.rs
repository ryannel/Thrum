//! W3 through CookedAssets, the production forward pass and its actual reactive reader.
#![cfg(target_os = "macos")]

use std::path::Path;
use std::time::{Duration, Instant};

use thrum_asset::Profile;
use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedSource, VolumePlacement};
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::present::SurfaceProfile;
use thrum_render::runtime::SubmittedImage;
use thrum_render::synthetic::{CaptureSource, DisplayOverlay};
use v1_game::{FrameConfig, FrameHost, GameSetup, SceneCamera, SurfaceOutput};

fn submitted(
    host: &mut FrameHost,
    device: &wgpu::Device,
    surface: &SurfaceOutput<'_>,
    clock: &mut Duration,
) -> SubmittedImage {
    *clock += Duration::from_millis(16);
    assert!(
        host.drive(*clock, Some(surface))
            .expect("ordinary frame")
            .submitted
    );
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("frame completion");
    while host.poll_completed().expect("retire frame").is_some() {}
    host.try_submitted_image()
        .expect("submitted readback")
        .expect("one image per frame")
}

// The graph witness textureLoads the actual R8Unorm attachment, then the temporal
// display diagnostic returns its red lane unchanged (no tone/gamma/dither). The
// intermediate fp16 roundtrip preserves every R8Unorm code exactly.
fn reactive(image: &SubmittedImage, x: usize, y: usize) -> u8 {
    assert_eq!(image.pixel_source, CaptureSource::Sdr);
    image.rgba[(y * image.extent[0] as usize + x) * 4]
}

fn retain(recipe: &str, image: &SubmittedImage, expected: u8, alpha: f64) {
    let Some(directory) = std::env::var_os("THRUM_M2_VOLUME_EVIDENCE_DIR") else {
        return;
    };
    let directory = Path::new(&directory);
    std::fs::create_dir_all(directory).expect("evidence directory");
    std::fs::write(directory.join(format!("{recipe}.rgba8")), &image.rgba).expect("raw witness");
    let record = serde_json::json!({
        "recipe": recipe, "extent": image.extent, "pixel_source": "Sdr/RGBA8 exact temporal diagnostic",
        "source_tick": image.source.tick, "device_generation": image.device_generation,
        "session": image.session, "expected_central_byte": expected,
        "analytic_alpha": alpha, "central_reactive": reactive(image, 32, 32),
        "uncovered_reactive": ([[0,0], [64,0], [0,64], [64,64]].map(|[x,y]| reactive(image,x,y))),
        "path": "Cooker -> CookedAssets -> Pipelines::draw -> production forward R8Unorm -> graph witness textureLoad -> submitted T2 copy"
    });
    std::fs::write(
        directory.join(format!("{recipe}.json")),
        serde_json::to_vec_pretty(&record).expect("record"),
    )
    .expect("evidence record");
}

#[test]
fn metal_cooked_volume_w3_uses_production_forward_reactive_attachment() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../crates/thrum-render/src/synthetic/temporal/witness.json"
    ))
    .expect("shared Chrome/Metal witness");
    let volume = &fixture["volume"];
    let extent: [u32; 3] = serde_json::from_value(volume["extent"].clone()).expect("volume extent");
    let density: u8 = serde_json::from_value(volume["density"].clone()).expect("density");
    let expected: u8 =
        serde_json::from_value(volume["expected_central_byte"].clone()).expect("W3 byte");
    let placement = VolumePlacement {
        name: volume["name"].as_str().expect("volume name").into(),
        transform: serde_json::from_value(volume["transform"].clone()).expect("transform"),
        extinction: serde_json::from_value(volume["extinction"].clone()).expect("extinction"),
        steps: serde_json::from_value(volume["steps"].clone()).expect("view steps"),
        light_steps: serde_json::from_value(volume["light_steps"].clone()).expect("light steps"),
    };
    let root =
        std::env::temp_dir().join(format!("thrum-m2-production-volume-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source")).expect("source directory");
    let mut tvol = b"TVOL".to_vec();
    for size in extent {
        tvol.extend(size.to_le_bytes());
    }
    tvol.resize(16 + extent.into_iter().product::<u32>() as usize, density);
    std::fs::write(
        root.join("source").join(format!("{}.tvol", placement.name)),
        tvol,
    )
    .expect("shared density");
    let mut cooker = Cooker::open(CookConfig {
        source: root.join("source"),
        store: root.join("store"),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("cooker");
    let report = cooker.cook_all().expect("cook volume");
    assert!(
        report.failed.is_empty(),
        "cook failures: {:?}",
        report.failed
    );

    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Metal adapter");
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Metal device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 1);
        for recipe in ["portable", "apple-tile"] {
            let caps = capabilities
                .clone()
                .with_recipe_override(recipe)
                .expect("Mac recipe");
            let view = device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("W3 offscreen presentation receiver, not physical output"),
                    size: wgpu::Extent3d {
                        width: 65,
                        height: 65,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default());
            let surface = SurfaceOutput {
                view: &view,
                extent: [65, 65],
            };
            let mut host = FrameHost::new_with_setup(
                &device,
                &queue,
                &caps,
                FrameConfig {
                    render_extent: [65, 65],
                    surface: Some(SurfaceProfile {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        extent: [65, 65],
                    }),
                },
                GameSetup {
                    display_overlay: DisplayOverlay::TemporalInputs,
                    ..GameSetup::default()
                },
            )
            .await
            .expect("real game host");
            host.pause().expect("steady scene");
            let center = glam::Mat4::from_cols_array_2d(&placement.transform)
                .transform_point3(glam::Vec3::splat(0.5));
            host.set_scene_camera(
                SceneCamera::perspective(
                    (center + glam::Vec3::Z * 3.0).to_array(),
                    center.to_array(),
                    46.0_f32.to_radians(),
                    0.1,
                    100.0,
                )
                .expect("central unit-length volume ray"),
            )
            .expect("fixture camera");
            let mut config = CookedConfig::new(CookedSource::Store(root.join("store")), Vec::new());
            config.volume = Some(placement.clone());
            let assets = CookedAssets::new(config).expect("production contribution");
            let stats = assets.stats();
            host.set_scene_contribution(Box::new(assets))
                .expect("attach cooked volume");
            host.enable_submitted_capture(1)
                .expect("submitted temporal diagnostic tap");
            let mut clock = Duration::ZERO;
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                let image = submitted(&mut host, &device, &surface, &mut clock);
                host.recycle_submitted_image(image);
                let stats = stats.lock().expect("cooked status");
                assert!(stats.errors.is_empty(), "{:?}", stats.errors);
                if stats.drawn == Some(1) {
                    assert!(
                        stats
                            .swaps
                            .last()
                            .expect("drawn publication")
                            .volume_profile
                            .is_some()
                    );
                    break;
                }
                assert!(Instant::now() < deadline, "volume never drawn");
                drop(stats);
                std::thread::yield_now();
            }
            // Two steady submitted frames eliminate initial scene/sky invalidation.
            let warm = submitted(&mut host, &device, &surface, &mut clock);
            host.recycle_submitted_image(warm);
            let image = submitted(&mut host, &device, &surface, &mut clock);
            assert_eq!(image.extent, [65, 65]);
            let alpha = 1.0 - (-f64::from(placement.extinction) * f64::from(density) / 255.0).exp();
            assert_eq!(
                (alpha * 255.0).round(),
                f64::from(expected),
                "shared W3 alpha quantization"
            );
            let actual = reactive(&image, 32, 32);
            assert_eq!(
                actual, expected,
                "W3 production reactive byte must equal volume alpha"
            );
            for [x, y] in [[0, 0], [64, 0], [0, 64], [64, 64]] {
                assert_eq!(
                    f64::from(reactive(&image, x, y)),
                    fixture["rows"]["W3"]["outside"]
                        .as_f64()
                        .expect("uncovered zero")
                );
            }
            println!(
                "M2 W3 {recipe}: production forward reactive={actual}, alpha={alpha}, expected byte={expected}, uncovered corners=0; submitted tick={}",
                image.source.tick
            );
            retain(recipe, &image, expected, alpha);
            host.recycle_submitted_image(image);
            host.disable_submitted_capture();
        }
    });
    std::fs::remove_dir_all(root).expect("remove owned fixture");
}
