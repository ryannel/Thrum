//! Explicit cooked-valley route proof, launched by run_v1_showcase.py culling.

use std::{io::Write, time::Duration};
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    synthetic::{CaptureSource, CullMode},
};
use v1_game::{CameraRoute, FrameConfig, FrameHost, FrameSample, SurfaceOutput};

fn complete(host: &mut FrameHost, device: &wgpu::Device) -> FrameSample {
    loop {
        if let Some(sample) = host.poll_completed().expect("poll") {
            return sample;
        }
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .expect("wait");
    }
}

pub(crate) fn run() {
    pollster::block_on(prove());
}

async fn prove() {
    let text = std::fs::read_to_string(std::env::var("THRUM_CAPTURE_ROUTE").expect("route"))
        .expect("route file");
    let (extent, overlay) = CameraRoute::declared(&text).expect("route declaration");
    let timing = std::env::var_os("THRUM_CULLING_TIMING").is_some();
    let capture_source = if std::env::var("THRUM_CULLING_SOURCE").as_deref() == Ok("scene-linear") {
        CaptureSource::SceneLinear
    } else {
        CaptureSource::Sdr
    };
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Metal");
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_timing_features(&adapter)
                | DeviceCapabilities::requested_content_features(&adapter),
            ..Default::default()
        })
        .await
        .expect("device");
    let caps = DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("valley oracle surface"),
        size: wgpu::Extent3d {
            width: extent[0],
            height: extent[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let surface = SurfaceOutput {
        view: &view,
        extent,
    };
    let mut modes = vec![CullMode::Direct, CullMode::Frustum, CullMode::Occlusion];
    if !timing {
        modes.push(CullMode::Direct);
    }
    let mut hosts = Vec::new();
    for &mode in &modes {
        let mut setup = v1_game::selected_setup().expect("setup");
        assert_eq!(
            setup.camera_culling,
            v1_game::GameSetup::valley_default_culling(),
            "launch default"
        );
        setup.camera_culling = mode;
        setup.route_overlay = overlay;
        let mut host = FrameHost::new_with_setup(
            &device,
            &queue,
            &caps,
            FrameConfig {
                render_extent: extent,
                surface: Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent,
                }),
            },
            setup,
        )
        .await
        .expect("host");
        assert_eq!(host.camera_culling(), mode, "resolved mode must match");
        let stats = v1_game::attach_cooked_scene(&mut host)
            .expect("attach")
            .expect("cooked valley");
        host.pause().expect("pause");
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        loop {
            drop(host.capture_pixels().expect("residency frame"));
            let stats = stats.lock().expect("stats");
            assert!(stats.errors.is_empty(), "{:?}", stats.errors);
            if stats.drawn == Some(1) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "valley residency deadline"
            );
        }
        host.resume(Duration::ZERO).expect("resume");
        hosts.push(host);
    }
    let route = CameraRoute::parse(&text, hosts[0].scene_camera().expect("valley camera"), 60)
        .expect("route");
    let last = route.origin_tick + route.keys.last().expect("key").tick;
    let output = std::path::PathBuf::from(std::env::var("THRUM_CULLING_OUTPUT").expect("output"));
    std::fs::create_dir_all(&output).expect("output directory");
    let mut log = std::fs::File::create(output.join("rows.jsonl")).expect("rows");
    writeln!(log, "{}", serde_json::json!({"adapter":adapter.get_info().name,"recipe":format!("{:?}",caps.recipe()),"extent":extent,"contended":true,"default":format!("{:?}",v1_game::GameSetup::valley_default_culling()),"load":std::process::Command::new("sysctl").args(["-n","vm.loadavg"]).output().ok().map(|o|String::from_utf8_lossy(&o.stdout).into_owned())})).expect("header");
    let mut hidden = [0_u64; 2];
    for tick in 1..=last {
        if tick == route.origin_tick {
            for host in &mut hosts {
                host.set_camera_route(route.clone()).expect("install route");
                if !timing {
                    host.enable_submitted_capture_from(1, capture_source)
                        .expect("enable T2 tap");
                }
            }
        }
        let mut images: Vec<Vec<u8>> = (0..modes.len()).map(|_| Vec::new()).collect();
        let count = modes.len();
        for index in (0..count).map(|step| (step + usize::try_from(tick).expect("small")) % count) {
            let host = &mut hosts[index];
            let now = Duration::from_secs_f64(host.route_step_ms(0.5) / 1000.0);
            let frame = host.drive(now, Some(&surface)).expect("route frame");
            assert_eq!(frame.tick, tick);
            assert!(frame.surface_presented && !frame.dropped_view && frame.completed_ticks == 1);
            let sample = complete(host, &device);
            if tick < route.origin_tick {
                continue;
            }
            let mut capture_identity = None;
            if !timing {
                let image = loop {
                    if let Some(image) = host.try_submitted_image().expect("tap") {
                        break image;
                    }
                    device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: Some(Duration::from_secs(30)),
                        })
                        .expect("image wait");
                };
                assert_eq!(image.source.tick, tick);
                assert_eq!(image.sample.submission_order, sample.submission_order);
                capture_identity = Some(
                    serde_json::json!({"source_tick":image.source.tick,"submission":sample.submission_order,"alpha_bits":sample.presentation_alpha.to_bits(),"exposure_gpu_words":image.exposure_gpu_words}),
                );
                if index == 1
                    && capture_source == CaptureSource::Sdr
                    && route
                        .checkpoints
                        .iter()
                        .any(|c| route.origin_tick + c.tick == tick)
                {
                    let mut ppm = format!("P6\n{} {}\n255\n", extent[0], extent[1]).into_bytes();
                    for pixel in image.rgba.as_chunks::<4>().0 {
                        ppm.extend_from_slice(&pixel[..3]);
                    }
                    std::fs::write(output.join(format!("fixed-{tick}.ppm")), ppm)
                        .expect("fixed submitted frame");
                }
                images[index].clone_from(&image.rgba);
                host.recycle_submitted_image(image);
            }
            if let Some(counts) = sample.camera_culling {
                assert_eq!(counts.mode, modes[index]);
                if index <= 2 {
                    hidden[index - 1] += u64::from(counts.drawn < counts.active);
                }
            }
            writeln!(log, "{}", serde_json::json!({"tick":tick,"mode":if index==3 {"DirectControl".to_owned()}else{format!("{:?}",modes[index])},"capture":capture_identity,"whole_gpu_ns":sample.gpu_pass_ns.as_ref().map(thrum_render::timing::SceneGpuTimings::sum),"counts":sample.camera_culling.map(|c|serde_json::json!({"active":c.active,"drawn":c.drawn,"prepass":c.prepass}))})).expect("row");
        }
        if tick >= route.origin_tick && !timing {
            assert_eq!(
                images[0].len(),
                (extent[0] * extent[1]) as usize
                    * usize::try_from(capture_source.bytes_per_pixel()).expect("pixel size")
            );
            for index in 1..count {
                if images[0] != images[index] {
                    for (i, image) in images.iter().enumerate() {
                        std::fs::write(
                            output.join(format!("failure-{tick}-{i}-{:?}.raw", modes[i])),
                            image,
                        )
                        .expect("failure pixels");
                    }
                    panic!("{:?} differs at submitted tick {tick}", modes[index]);
                }
            }
        }
    }
    assert!(
        hidden.into_iter().all(|ticks| ticks > 0),
        "both modes must hide work"
    );
    writeln!(
        log,
        "{}",
        serde_json::json!({"compared":if timing {0} else {last-route.origin_tick+1},"timed":timing,"hidden_ticks":hidden,"equal":!timing})
    )
    .expect("result");
}
