use crate::support;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{
        CaptureLimits, ExtentRequest, ExtentTransition, FrameConfig, FrameError, FrameSample,
        PreparedExtent, SurfaceOutput, SurfaceSwitchOutcome,
    },
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene},
};

const OLD: [u32; 2] = [64, 64];
const STALE: [u32; 2] = [80, 64];
const NEW: [u32; 2] = [96, 64];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn request(extent: [u32; 2], device: u64) -> ExtentRequest {
    ExtentRequest {
        extent,
        backing_scale: 2_f64.to_bits(),
        format: FORMAT,
        device,
    }
}

fn host(capabilities: &DeviceCapabilities, queue: &wgpu::Queue, extent: [u32; 2]) -> support::Host {
    let mut host = pollster::block_on(support::host_with_config(
        capabilities,
        queue,
        SyntheticScene::grid(1).expect("test scene"),
        FrameLighting::Diagnostic(SyntheticLight::default()),
        FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile {
                format: FORMAT,
                extent,
            }),
        },
    ));
    host.set_timing_requested(false);
    host
}

fn view(device: &wgpu::Device, extent: [u32; 2]) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("transition surface double"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn submit(host: &mut support::Host, view: &wgpu::TextureView, extent: [u32; 2]) {
    let report = host
        .drive(Duration::ZERO, Some(&SurfaceOutput { view, extent }))
        .expect("transition fixture");
    assert!(report.submitted && report.surface_presented);
}

fn complete(host: &mut support::Host, device: &wgpu::Device) -> FrameSample {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        device
            .poll(wgpu::PollType::Poll)
            .expect("transition fixture");
        if let Some(sample) = host.poll_completed().expect("transition fixture") {
            return sample;
        }
        assert!(Instant::now() < deadline, "no completed Metal frame");
        std::thread::yield_now();
    }
}

type ExtentJob = fn(&AtomicBool) -> Result<Option<PreparedExtent>, FrameError>;

fn unexpected_build() -> Result<ExtentJob, FrameError> {
    panic!("request was not yet eligible to build");
}

fn prepare(
    host: &support::Host,
    extent: [u32; 2],
) -> impl FnOnce(&AtomicBool) -> Result<Option<PreparedExtent>, FrameError> + Send + use<> {
    let inputs = host
        .extent_prepare_inputs(extent)
        .expect("transition fixture");
    move |cancel| inputs.build(cancel)
}

#[test]
fn mac_host_holds_old_frames_and_only_publishes_latest_generation() {
    let Some(gpu) =
        thrum_test_support::gpu("extent generation transition", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let mut host = host(&cap, &queue, OLD);
    host.enable_submitted_capture_with_limits(7, CaptureLimits::default())
        .expect("transition fixture");
    let old_output = view(&device, OLD);
    let new_output = view(&device, NEW);
    let old = request(OLD, 1);
    let stale = request(STALE, 1);
    let new = request(NEW, 1);
    let mut transition = ExtentTransition::new(old);
    let ms = Duration::from_millis;
    transition
        .advance(stale, old, ms(0), unexpected_build)
        .expect("transition fixture");
    transition
        .advance(stale, old, ms(99), unexpected_build)
        .expect("transition fixture");
    let (release, blocked) = mpsc::channel();
    let first_build = prepare(&host, STALE);
    transition
        .advance(stale, old, ms(100), || {
            Ok(move |cancel: &AtomicBool| {
                // Hold a finished result on the preparation thread until it is superseded.
                let ready = first_build(cancel)?;
                blocked
                    .recv_timeout(Duration::from_secs(10))
                    .expect("transition fixture");
                assert!(cancel.load(Ordering::Acquire));
                drop(ready);
                Ok(None)
            })
        })
        .expect("transition fixture");

    submit(&mut host, &old_output, OLD);
    let first = complete(&mut host, &device);
    transition
        .advance(new, old, ms(102), unexpected_build)
        .expect("transition fixture");
    submit(&mut host, &old_output, OLD);
    let held = complete(&mut host, &device);
    assert_eq!(held.render_extent, OLD);
    assert_eq!(held.history_identity[1], first.history_identity[0]);
    assert!(transition.ready().is_none());
    assert_eq!(transition.surface.record().configure_calls, 0);
    release.send(()).expect("transition fixture");

    let deadline = Instant::now() + Duration::from_secs(10);
    let candidate = loop {
        transition
            .advance(new, old, ms(202), || Ok(prepare(&host, NEW)))
            .expect("transition fixture");
        if let Some(ready) = transition.ready() {
            break ready;
        }
        assert!(
            Instant::now() < deadline,
            "latest build did not become ready"
        );
        std::thread::yield_now();
    };
    assert!(host.admits_extent(&candidate));
    assert_eq!(
        transition
            .surface
            .take(true, false, &NEW, &OLD, |_| -> Result<(), &str> {
                panic!("a still-held drawable must not configure")
            }),
        SurfaceSwitchOutcome::Deferred,
    );
    transition.defer(candidate);
    submit(&mut host, &old_output, OLD);
    assert_eq!(host.pending_frames(), 1);
    let candidate = transition.ready().expect("transition fixture");
    assert_eq!(
        transition
            .surface
            .take(false, transition.busy(), &NEW, &OLD, |extent| {
                assert_eq!(*extent, NEW);
                // Configure on released wgpu waits for this frame.
                device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("transition fixture");
                Ok::<_, &str>(())
            }),
        SurfaceSwitchOutcome::Taken
    );
    assert!(host.publish_extent(candidate));
    assert_eq!(transition.surface.record().generations_taken, 1);
    assert_eq!(transition.surface.record().deferrals, 1);
    assert_eq!(
        host.pending_frames(),
        1,
        "old slot survives for its completion"
    );
    let last_old = complete(&mut host, &device);
    assert_eq!(last_old.render_extent, OLD);
    assert_eq!(last_old.history_identity[1], held.history_identity[0]);

    submit(&mut host, &new_output, NEW);
    let switched = complete(&mut host, &device);
    assert_eq!(switched.render_extent, NEW);
    let mut independent = self::host(&cap, &queue, NEW);
    independent.invalidate_history();
    submit(&mut independent, &new_output, NEW);
    let reset = complete(&mut independent, &device);
    assert_eq!(switched.history_identity[1], reset.history_identity[1]);
    submit(&mut host, &new_output, NEW);
    let resumed = complete(&mut host, &device);
    assert_eq!(resumed.history_identity[1], switched.history_identity[0]);
    assert_ne!(resumed.history_identity[1], reset.history_identity[1]);

    let deadline = Instant::now() + Duration::from_secs(5);
    let expected = [first, held, last_old, switched, resumed];
    for expected in expected {
        let image = loop {
            device
                .poll(wgpu::PollType::Poll)
                .expect("transition fixture");
            if let Some(image) = host.try_submitted_image().expect("transition fixture") {
                break image;
            }
            assert!(
                Instant::now() < deadline,
                "capture did not drain through the switch"
            );
            std::thread::yield_now();
        };
        assert_eq!(image.session, 7);
        assert_eq!(image.sample.submission_order, expected.submission_order);
        assert_eq!(image.extent, expected.render_extent);
        assert_eq!(image.sample.history_identity, expected.history_identity);
        host.recycle_submitted_image(image);
    }
    assert_eq!(host.submitted_capture_omissions(), 0);

    let report = host.creation_report();
    let build_label = "prepare scene extent";
    assert!(
        report
            .rows
            .iter()
            .any(|row| row.label == build_label && row.counts.textures > 0)
    );
    let wrong_thread = std::thread::current().name().unwrap_or_default().to_owned();
    assert!(!report.rows.iter().any(|row| row.label == build_label
        && row.thread == wrong_thread
        && row.counts.textures > 0));
    let control = prepare(&host, STALE)(&AtomicBool::new(false)).expect("transition fixture");
    let report = host.creation_report();
    assert!(
        report.rows.iter().any(|row| row.label == build_label
            && row.thread == wrong_thread
            && row.counts.textures > 0),
        "creation accounting failed to detect the render-thread control: {:?}",
        report.rows
    );
    drop(control);
}

#[test]
fn failed_and_lost_builds_never_take_the_surface() {
    let Some(gpu) = thrum_test_support::gpu("failed extent generation", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let restriction = thrum_test_support::Restriction::default();
    let (device, queue) = gpu.device(thrum_test_support::Profile::Portable, &restriction);
    let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let mut host = host(&cap, &queue, OLD);
    let old_output = view(&device, OLD);
    let old = request(OLD, 1);
    let stale = request(STALE, 1);
    let new = request(NEW, 1);
    let mut transition = ExtentTransition::new(old);
    let ms = Duration::from_millis;

    transition.observe(stale, old, ms(0));
    transition
        .advance(stale, old, ms(100), || {
            Ok(|_: &AtomicBool| Err(FrameError::Gpu("forced allocation failure".into())))
        })
        .expect("transition fixture");
    let deadline = Instant::now() + Duration::from_secs(5);
    let failure = loop {
        if let Err(error) = transition.advance(stale, old, ms(101), unexpected_build) {
            break error;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert!(failure.to_string().contains("forced allocation failure"));
    transition
        .advance(stale, old, ms(900), unexpected_build)
        .expect("transition fixture");
    assert!(transition.ready().is_none());
    assert_eq!(transition.surface.record().configure_calls, 0);
    submit(&mut host, &old_output, OLD);
    assert_eq!(complete(&mut host, &device).render_extent, OLD);

    let inputs = host.device_prepare_inputs().expect("transition fixture");
    let job = prepare(&host, NEW);
    let (finished, pending) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    transition.observe(new, old, ms(1000));
    transition
        .advance(new, old, ms(1100), || {
            Ok(move |cancel: &AtomicBool| {
                let value = job(cancel)?;
                finished.send(()).expect("transition fixture");
                blocked
                    .recv_timeout(Duration::from_secs(5))
                    .expect("transition fixture");
                assert!(cancel.load(Ordering::Acquire));
                Ok(value)
            })
        })
        .expect("transition fixture");
    pending
        .recv_timeout(Duration::from_secs(5))
        .expect("transition fixture");
    device.destroy();
    transition.cancel();
    release.send(()).expect("transition fixture");
    let (device, queue) = gpu.device(thrum_test_support::Profile::Portable, &restriction);
    let new_cap = DeviceCapabilities::capture(&gpu.adapter, &device, 2);
    let mut recovered = pollster::block_on(inputs.prepare(
        &new_cap,
        &queue,
        Some(SurfaceProfile {
            format: FORMAT,
            extent: OLD,
        }),
    ))
    .expect("transition fixture");
    host.publish_device(&mut recovered)
        .expect("transition fixture");
    let old = request(OLD, 2);
    let new = request(NEW, 2);
    transition.recovered(old, true);
    transition.observe(new, old, ms(1200));
    let deadline = Instant::now() + Duration::from_secs(5);
    let ready = loop {
        transition
            .advance(new, old, ms(1300), || Ok(prepare(&host, NEW)))
            .expect("transition fixture");
        if let Some(ready) = transition.ready() {
            break ready;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert_eq!(transition.surface.record().configure_calls, 0);
    assert!(host.admits_extent(&ready));
    assert!(host.publish_extent(ready));
    let new_output = view(&device, NEW);
    submit(&mut host, &new_output, NEW);
    assert_eq!(complete(&mut host, &device).render_extent, NEW);
}
