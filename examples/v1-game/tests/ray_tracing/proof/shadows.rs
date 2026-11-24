//! Compare feature selections at the same prepared valley samples.

use super::{demand::valley_store, *};
use thrum_render::runtime::SurfaceOutput;
use thrum_render::synthetic::{CaptureSource, SunShadowMethod};
use v1_game::CameraRoute;

const ROUTE: &str = include_str!("../../../routes/moving-v1.json");

fn valley(
    store: &Path,
    metal: &Metal,
    method: SunShadowMethod,
    lamps: bool,
    extent: [u32; 2],
    hour: f32,
    route_overlay: v1_game::RouteOverlay,
    penumbra: bool,
) -> (Host, Stats) {
    let setup = GameSetup {
        sun_shadow_method: method,
        sun_penumbra: penumbra,
        lamp_shadows: lamps,
        valley_stars: true,
        route_overlay,
        ..GameSetup::default()
    };
    let config = FrameConfig {
        render_extent: extent,
        surface: Some(thrum_render::present::SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent,
        }),
    };
    let mut host =
        pollster::block_on(setup.prepare(&metal.device, &metal.queue, &metal.caps, config))
            .expect("valley host");
    let (camera, assets) =
        v1_game::prepare_cooked_valley(store.to_owned().into()).expect("cooked valley");
    let stats = assets.stats();
    host.set_scene_camera(camera).expect("authored camera");
    host.set_time_of_day(hour / 24.0).expect("fixed Sun");
    host.set_game_contribution(Box::new(assets))
        .expect("prepared valley");
    host.world_mut().pause().expect("held scene before route");
    drop(settle(&mut host, &stats, generation(store)));
    (host, stats)
}

fn surface(device: &wgpu::Device, extent: [u32; 2]) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("paired valley frame surface"),
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
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn frame(
    host: &mut Host,
    metal: &Metal,
    now: Duration,
    surface: &SurfaceOutput<'_>,
) -> thrum_render::runtime::FrameSample {
    assert!(
        host.drive(now, Some(surface))
            .expect("submit valley")
            .submitted
    );
    wait(&metal.device);
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(sample) = host.poll_completed().expect("complete valley") {
            return sample;
        }
        assert!(Instant::now() < deadline, "valley frame did not complete");
        std::thread::yield_now();
    }
}

pub(crate) fn prove_sun_out() {
    let Some(metal) = Metal::new(1) else {
        return;
    };
    let store = valley_store("sun-out");
    let drawable = surface(&metal.device, EXTENT);
    let surface = SurfaceOutput {
        view: &drawable,
        extent: EXTENT,
    };
    let mut reference = None;
    for method in [SunShadowMethod::Maps, SunShadowMethod::Traced] {
        let (mut host, _) = valley(
            &store,
            &metal,
            method,
            false,
            EXTENT,
            0.0,
            v1_game::RouteOverlay::Off,
            false,
        );
        host.enable_submitted_capture_from(
            1,
            thrum_render::runtime::CaptureLimits::default(),
            CaptureSource::SceneLinear,
        )
        .expect("linear T2 tap");
        let sample = frame(&mut host, &metal, Duration::ZERO, &surface);
        assert_eq!(sample.shadow_selection.sun, method);
        let deadline = Instant::now() + Duration::from_secs(20);
        let image = loop {
            if let Some(image) = host.try_submitted_image().expect("T2 readback") {
                break image;
            }
            assert!(
                Instant::now() < deadline,
                "scene-linear bytes did not complete"
            );
            wait(&metal.device);
        };
        assert_eq!(image.sample.submission_order, sample.submission_order);
        if let Some(reference) = &reference {
            assert!(
                &image.rgba == reference,
                "with the valley Sun below the horizon, scene-linear half-float bytes must match exactly"
            );
        } else {
            assert!(
                image
                    .rgba
                    .as_chunks::<8>()
                    .0
                    .windows(2)
                    .any(|pair| pair[0] != pair[1])
            );
            reference = Some(image.rgba.clone());
        }
        report(
            &json!({"method":format!("{method:?}"),"solar_hour":0,"tolerance_half_float_ulps":0,
            "scene_linear_bytes":image.rgba.len(),"image_digest":digest(&image.rgba),"target":"Apple M3 Max Metal"}),
        );
    }
    std::fs::remove_dir_all(store).expect("remove owned cooked fixture");
}

pub(crate) fn measure_shadows() {
    let Some(metal) = Metal::new(1) else {
        return;
    };
    let store = valley_store("shadows-cost");
    let extent = [1280, 720];
    let drawable = surface(&metal.device, extent);
    let surface = SurfaceOutput {
        view: &drawable,
        extent,
    };
    let cooked_root = AssetStore::new(&store)
        .read_head()
        .expect("read the valley store")
        .expect("published valley")
        .manifest
        .to_string();
    let profile_soft = std::env::var_os("THRUM_RAY_SOFT_COST").is_some();
    let selections: &[_] = if profile_soft {
        &[
            (SunShadowMethod::Traced, false, false),
            (SunShadowMethod::Traced, false, true),
        ]
    } else {
        &[
            (SunShadowMethod::Maps, false, false),
            (SunShadowMethod::Traced, false, false),
            (SunShadowMethod::Traced, true, false),
        ]
    };
    for &(method, lamps, penumbra) in selections {
        let (mut host, stats) = valley(
            &store,
            &metal,
            method,
            lamps,
            extent,
            17.3,
            v1_game::RouteOverlay::Proof,
            profile_soft,
        );
        if profile_soft {
            host.set_quality_knob("Sun penumbra", u32::from(penumbra))
                .expect("prepared Sun penumbra");
        }
        let history_bytes = host.sun_penumbra_history_bytes();
        let mut input: Value = serde_json::from_str(ROUTE).expect("authored moving route");
        input["cooked_root"] = json!(cooked_root);
        let route =
            CameraRoute::parse(&input.to_string(), host.scene_camera().expect("camera"), 60)
                .expect("validated route");
        host.world_mut()
            .resume(Duration::ZERO)
            .expect("start route");
        while host.world().tick() < route.origin_tick - 1 {
            let now = Duration::from_secs_f64(host.route_step_ms(0.5) / 1000.0);
            host.advance_without_rendering(now)
                .expect("approach route origin");
        }
        host.set_camera_route(route.clone()).expect("arm route");
        for tick in 0..=route.keys.last().expect("route end").tick {
            let now = Duration::from_secs_f64(host.route_step_ms(0.5) / 1000.0);
            if profile_soft && penumbra && tick == 180 {
                assert!(
                    host.drive(now, Some(&surface))
                        .expect("submit dive measurement")
                        .submitted
                );
                let texture = host
                    .submitted_sun_disc_witness()
                    .expect("raw disc ray results");
                let bytes = thrum_test_support::readback::texture(
                    &metal.device,
                    &metal.queue,
                    texture,
                    thrum_test_support::readback::Mip::level(0),
                    ..,
                );
                let mut counts = [0_u32; 4];
                let mut aligned = 0_u32;
                for [_, _, valid, dive] in bytes.as_chunks::<4>().0 {
                    aligned += u32::from(dive & 4 != 0);
                    counts[match (valid, dive & 3) {
                        (1, 0) => 0,
                        (1, 1) => 1,
                        (0, 2) => 2,
                        _ => 3,
                    }] += 1;
                }
                report(
                    &json!({"type":"dive_account","tick":tick,"ordinary":counts[0],
                    "dipping":counts[1],"excluded":counts[2],"background_or_invalid":counts[3],"depth_alignment_queries":aligned,
                    "target":"mac-metal"}),
                );
                wait(&metal.device);
                drop(thrum_test_support::wait::until(
                    "complete dive measurement",
                    Duration::from_secs(20),
                    || host.poll_completed().expect("complete dive measurement"),
                ));
                continue;
            }
            let sample = frame(&mut host, &metal, now, &surface);
            assert_eq!(sample.tick, route.origin_tick + tick);
            assert_eq!(sample.shadow_selection.sun, method);
            assert_eq!(sample.shadow_selection.lamps, lamps);
            if (profile_soft && tick == 180)
                || ![0, 90, 180, 230, 290, 340, 360]
                    .iter()
                    .any(|stop| (*stop..=*stop + 3).contains(&tick))
            {
                continue;
            }
            let timings = sample.gpu_pass_ns.expect("Metal timestamps");
            let span = timings
                .timed_pass_span_nanoseconds()
                .expect("timed-pass span");
            let passes = timings.values();
            let dive_limit = host
                .quality_knobs()
                .into_iter()
                .find(|knob| knob.name == "Sun penumbra dive limit")
                .map(|knob| knob.value);
            report(
                &json!({"method":format!("{method:?}"),"lamps":lamps,"penumbra":penumbra,"dive_limit_degrees":dive_limit,"history_bytes":history_bytes,"tick":tick,"timed_pass_span_ns":span,
                "frame_cpu_ns":(sample.upload_cpu+sample.encode_cpu).as_nanos(),"passes_ns":passes,
                "upload_cpu_ns":sample.upload_cpu.as_nanos(),"encode_cpu_ns":sample.encode_cpu.as_nanos(),
                "recipe":"AppleTile","cooked_root":cooked_root,"resolved_draw_path":format!("{:?}",stats.lock().expect("stats").draw_path)}),
            );
        }
    }
    std::fs::remove_dir_all(store).expect("remove owned cooked fixture");
}
