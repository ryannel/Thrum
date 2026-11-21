//! Explicit cooked-valley route proof, launched by run_v1_showcase.py culling.

use std::{
    io::{Read, Write},
    time::Duration,
};
use thrum_render::{
    capabilities::{DeviceRecipe, OpenedDevice},
    present::SurfaceProfile,
    runtime::SubmittedImage,
    synthetic::{CaptureSource, CullMode},
};
use v1_game::{
    CameraRoute, FrameConfig, FrameHost, FrameSample, RouteOverlay, SceneCamera, SurfaceOutput,
};

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

fn submitted_image(host: &mut FrameHost, device: &wgpu::Device) -> SubmittedImage {
    loop {
        if let Some(image) = host.try_submitted_image().expect("tap") {
            return image;
        }
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .expect("image wait");
    }
}

pub(crate) fn run(test: &str) {
    let Some(gpu) = thrum_test_support::gpu(test, Some(wgpu::Backend::Metal)) else {
        return;
    };
    pollster::block_on(prove(&gpu));
}

// Failure-only diagnosis. The first comparison and its bytes remain unchanged.
// Repeating the same elapsed time distinguishes cross-host differences from
// variation on one prepared host; receipts expose any changed alpha/exposure.
fn recapture_failure(
    hosts: &mut [FrameHost],
    device: &wgpu::Device,
    surface: &SurfaceOutput<'_>,
    times: &[Duration],
    modes: &[CullMode],
    output: &std::path::Path,
    tick: u64,
) {
    let mut log = std::fs::File::create(output.join("failure-recaptures.jsonl"))
        .expect("failure recapture log");
    for repeat in 0..3 {
        for (index, host) in hosts.iter_mut().enumerate() {
            let frame = host
                .drive(times[index], Some(surface))
                .expect("repeat frame");
            let sample = complete(host, device);
            let image = submitted_image(host, device);
            let file = format!(
                "failure-{tick}-repeat-{repeat}-{index}-{:?}.raw",
                modes[index]
            );
            std::fs::write(output.join(&file), &image.rgba).expect("repeat bytes");
            writeln!(log, "{}", serde_json::json!({"file":file,"repeat":repeat,"host":index,"mode":format!("{:?}",modes[index]),"requested_elapsed_ns":times[index].as_nanos(),"tick":frame.tick,"completed_ticks":frame.completed_ticks,"source_tick":image.source.tick,"submission":sample.submission_order,"alpha_bits":sample.presentation_alpha.to_bits(),"exposure_gpu_words":image.exposure_gpu_words})).expect("repeat receipt");
            host.recycle_submitted_image(image);
        }
    }
}

fn record_sample(
    log: &mut std::fs::File,
    tick: u64,
    mode: &str,
    capture: Option<&serde_json::Value>,
    sample: &FrameSample,
) {
    let pass_ns = sample.gpu_pass_ns.as_ref().map(|timings| {
        let mut totals = std::collections::BTreeMap::<String, f64>::new();
        for pass in timings.passes() {
            *totals.entry(pass.name.to_string()).or_default() += pass.nanoseconds;
        }
        totals
    });
    let unmeasured = sample.gpu_pass_ns.as_ref().map(|timings| {
        timings
            .values()
            .into_iter()
            .filter(|(_, time)| time.is_none())
            .map(|(name, _)| name.to_owned())
            .collect::<Vec<_>>()
    });
    writeln!(log, "{}", serde_json::json!({"tick":tick,"mode":mode,"capture":capture,"whole_gpu_ns":sample.gpu_pass_ns.as_ref().map(thrum_render::timing::SceneGpuTimings::sum),"pass_gpu_ns":pass_ns,"unmeasured_gpu_passes":unmeasured,"counts":sample.camera_culling.map(|c|serde_json::json!({"active":c.active,"drawn":c.drawn,"prepass":c.prepass}))})).expect("row");
}

fn surface_texture(device: &wgpu::Device, extent: [u32; 2]) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
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
    })
}

fn load_valley(host: &mut FrameHost) {
    let stats = v1_game::attach_cooked_scene(host)
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
}

fn parse_route(text: &str, camera: SceneCamera) -> CameraRoute {
    if std::env::var("THRUM_SCENE").as_deref() == Ok("exit") {
        CameraRoute::parse_long_capture(text, camera, 60).expect("exit route")
    } else {
        CameraRoute::parse(text, camera, 60).expect("valley route")
    }
}

enum ExitReference {
    Write(std::fs::File),
    Compare {
        file: std::fs::File,
        pixels: Vec<u8>,
    },
}

struct ExitArm {
    name: String,
    mode: CullMode,
    reference: ExitReference,
}

impl ExitArm {
    fn from_env() -> Option<Self> {
        let name = std::env::var("THRUM_CULLING_EXIT_ARM").ok()?;
        let path = std::env::var_os("THRUM_CULLING_EXIT_REFERENCE").expect("Direct stream path");
        let mode = match name.as_str() {
            "Direct" | "DirectControl" => CullMode::Direct,
            "Frustum" => CullMode::Frustum,
            "Occlusion" => CullMode::Occlusion,
            _ => panic!("unknown exit oracle arm {name}"),
        };
        let reference = if name == "Direct" {
            ExitReference::Write(std::fs::File::create_new(path).expect("create Direct stream"))
        } else {
            ExitReference::Compare {
                file: std::fs::File::open(path).expect("open Direct stream"),
                pixels: Vec::new(),
            }
        };
        Some(Self {
            name,
            mode,
            reference,
        })
    }

    fn difference(&mut self, submitted: &[u8]) -> Option<&[u8]> {
        match &mut self.reference {
            ExitReference::Write(file) => {
                file.write_all(submitted).expect("write Direct pixels");
                None
            }
            ExitReference::Compare { file, pixels } => {
                pixels.resize(submitted.len(), 0);
                file.read_exact(pixels).expect("read complete Direct frame");
                (pixels != submitted).then_some(pixels.as_slice())
            }
        }
    }

    fn finish(&mut self, frames: u64) -> u64 {
        match &mut self.reference {
            ExitReference::Write(file) => {
                file.flush().expect("flush Direct stream");
                0
            }
            ExitReference::Compare { file, .. } => {
                assert_eq!(file.read(&mut [0]).expect("Direct stream tail"), 0);
                frames
            }
        }
    }
}

async fn prepare_hosts(
    opened: &OpenedDevice,
    extent: [u32; 2],
    overlay: RouteOverlay,
    timing: bool,
    exit_arm: Option<&ExitArm>,
) -> (Vec<CullMode>, Vec<FrameHost>) {
    let mut modes = vec![CullMode::Direct, CullMode::Frustum, CullMode::Occlusion];
    if let Some(arm) = exit_arm {
        modes = vec![arm.mode];
    } else if !timing {
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
            &opened.device,
            &opened.queue,
            &opened.capabilities,
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
        load_valley(&mut host);
        hosts.push(host);
    }
    (modes, hosts)
}

async fn prove(gpu: &thrum_test_support::Gpu) {
    let text = std::fs::read_to_string(std::env::var("THRUM_CAPTURE_ROUTE").expect("route"))
        .expect("route file");
    let (extent, overlay) = CameraRoute::declared(&text).expect("route declaration");
    let timing = std::env::var_os("THRUM_CULLING_TIMING").is_some();
    let mut exit_arm = ExitArm::from_env();
    assert!(
        exit_arm.is_none()
            || (!timing && std::env::var("THRUM_SCENE").as_deref() == Ok("exit"))
    );
    let capture_source = if std::env::var("THRUM_CULLING_SOURCE").as_deref() == Ok("scene-linear") {
        CaptureSource::SceneLinear
    } else {
        CaptureSource::Sdr
    };
    let opened = DeviceRecipe::default()
        .open_admitted(&gpu.instance, &gpu.adapter)
        .await
        .expect("Metal device for valley culling");
    let (modes, mut hosts) =
        prepare_hosts(&opened, extent, overlay, timing, exit_arm.as_ref()).await;
    let (device, caps) = (opened.device, opened.capabilities);
    let texture = surface_texture(&device, extent);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let surface = SurfaceOutput {
        view: &view,
        extent,
    };
    let route = parse_route(&text, hosts[0].scene_camera().expect("cooked scene camera"));
    let last = route.origin_tick + route.keys.last().expect("key").tick;
    let output = std::path::PathBuf::from(std::env::var("THRUM_CULLING_OUTPUT").expect("output"));
    std::fs::create_dir_all(&output).expect("output directory");
    let mut log = std::fs::File::create(output.join("rows.jsonl")).expect("rows");
    writeln!(log, "{}", serde_json::json!({"adapter":caps.adapter().name,"recipe":format!("{:?}",caps.recipe()),"extent":extent,"contended":true,"mode":"fixed","capture_source":format!("{capture_source:?}"),"timing_only":timing,"default":format!("{:?}",v1_game::GameSetup::valley_default_culling()),"load":std::process::Command::new("sysctl").args(["-n","vm.loadavg"]).output().ok().map(|o|String::from_utf8_lossy(&o.stdout).into_owned())})).expect("header");
    let mut hidden = [0_u64; 2];
    let mut last_times = vec![Duration::ZERO; modes.len()];
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
            last_times[index] = now;
            let frame = host.drive(now, Some(&surface)).expect("route frame");
            assert_eq!(frame.tick, tick);
            assert!(frame.surface_presented && !frame.dropped_view && frame.completed_ticks == 1);
            let sample = complete(host, &device);
            if tick < route.origin_tick {
                continue;
            }
            let mut capture_identity = None;
            if !timing {
                let image = submitted_image(host, &device);
                assert_eq!(image.source.tick, tick);
                assert_eq!(image.sample.submission_order, sample.submission_order);
                capture_identity = Some(
                    serde_json::json!({"source_tick":image.source.tick,"submission":sample.submission_order,"alpha_bits":sample.presentation_alpha.to_bits(),"exposure_gpu_words":image.exposure_gpu_words}),
                );
                if modes[index] == CullMode::Frustum
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
                match modes[index] {
                    CullMode::Frustum => hidden[0] += u64::from(counts.drawn < counts.active),
                    CullMode::Occlusion => hidden[1] += u64::from(counts.drawn < counts.active),
                    CullMode::Direct => {}
                }
            }
            let mode = if let Some(arm) = &exit_arm {
                arm.name.clone()
            } else if index == 3 {
                "DirectControl".to_owned()
            } else {
                format!("{:?}", modes[index])
            };
            record_sample(
                &mut log,
                tick,
                &mode,
                capture_identity.as_ref(),
                &sample,
            );
        }
        if tick >= route.origin_tick && !timing {
            assert_eq!(
                images[0].len(),
                (extent[0] * extent[1]) as usize
                    * usize::try_from(capture_source.bytes_per_pixel()).expect("pixel size")
            );
            if let Some(arm) = &mut exit_arm {
                if let Some(reference) = arm.difference(&images[0]) {
                    std::fs::write(output.join(format!("failure-{tick}-Direct.raw")), reference)
                        .expect("Direct failure pixels");
                    std::fs::write(
                        output.join(format!("failure-{tick}-{}.raw", arm.name)),
                        &images[0],
                    )
                    .expect("candidate failure pixels");
                    recapture_failure(
                        &mut hosts,
                        &device,
                        &surface,
                        &last_times,
                        &modes,
                        &output,
                        tick,
                    );
                    panic!("{} differs at submitted tick {tick}", arm.name);
                }
                continue;
            }
            for index in 1..count {
                if images[0] != images[index] {
                    for (i, image) in images.iter().enumerate() {
                        std::fs::write(
                            output.join(format!("failure-{tick}-{i}-{:?}.raw", modes[i])),
                            image,
                        )
                        .expect("failure pixels");
                    }
                    recapture_failure(
                        &mut hosts,
                        &device,
                        &surface,
                        &last_times,
                        &modes,
                        &output,
                        tick,
                    );
                    panic!("{:?} differs at submitted tick {tick}", modes[index]);
                }
            }
        }
    }
    let frames = last - route.origin_tick + 1;
    if let Some(arm) = &mut exit_arm {
        let compared = arm.finish(frames);
        writeln!(log, "{}", serde_json::json!({"frames":frames,"compared":compared,"timed":false,"hidden_ticks":hidden,"hosts":1,"differing_ticks":0,"equal":true})).expect("exit arm result");
        return;
    }
    assert!(
        hidden.into_iter().all(|ticks| ticks > 0),
        "both modes must hide work"
    );
    writeln!(
        log,
        "{}",
        serde_json::json!({"compared":if timing {0} else {last-route.origin_tick+1},"timed":timing,"hidden_ticks":hidden,"hosts":modes.len(),"differing_ticks":0,"equal":!timing})
    )
    .expect("result");
}
