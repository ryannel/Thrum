#[cfg(target_os = "macos")]
use crate::support;
#[cfg(target_os = "macos")]
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
#[cfg(target_os = "macos")]
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{
        CaptureLimits, ExtentRequest, ExtentTransition, FrameConfig, FrameError, FrameSample,
        PreparedExtent, SubmittedImage, SurfaceOutput, SurfaceSwitchOutcome,
    },
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene},
};
#[cfg(target_os = "macos")]
use tracing::{Dispatch, Subscriber, span};
#[cfg(target_os = "macos")]
use tracing_subscriber::{Layer, layer::Context, prelude::*};

#[cfg(target_os = "macos")]
const OLD: [u32; 2] = [64, 64];
#[cfg(target_os = "macos")]
const STALE: [u32; 2] = [80, 64];
#[cfg(target_os = "macos")]
const NEW: [u32; 2] = [96, 64];
#[cfg(target_os = "macos")]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
// The device-loss arm tears down a Metal device while work is held. Run these
// proofs one at a time so overlapping test devices cannot disturb the loss
// and capture observations.
#[cfg(target_os = "macos")]
static METAL: Mutex<()> = Mutex::new(());

#[derive(Clone, Default)]
#[cfg(target_os = "macos")]
struct Creations(Arc<Mutex<Vec<(std::thread::ThreadId, &'static str)>>>);

#[cfg(target_os = "macos")]
impl<S: Subscriber> Layer<S> for Creations {
    fn on_new_span(&self, attributes: &span::Attributes<'_>, _: &span::Id, _: Context<'_, S>) {
        let metadata = attributes.metadata();
        if metadata.target().starts_with("wgpu_core::")
            && matches!(
                metadata.name(),
                "Device::create_texture"
                    | "Device::create_texture_from_hal"
                    | "Texture::create_view"
                    | "Device::create_bind_group"
                    | "Device::create_general_render_pipeline"
                    | "Device::create_compute_pipeline"
            )
        {
            self.0
                .lock()
                .expect("creation event")
                .push((std::thread::current().id(), metadata.name()));
        }
    }
}

#[cfg(target_os = "macos")]
impl Creations {
    fn on(&self, thread: std::thread::ThreadId) -> Vec<&'static str> {
        self.0
            .lock()
            .expect("creation events")
            .iter()
            .filter_map(|&(creator, kind)| (creator == thread).then_some(kind))
            .collect()
    }

    fn dispatch(&self) -> Dispatch {
        Dispatch::new(tracing_subscriber::registry().with(self.clone()))
    }
}

#[cfg(target_os = "macos")]
fn request(extent: [u32; 2], device: u64) -> ExtentRequest {
    ExtentRequest {
        extent,
        backing_scale: 2_f64.to_bits(),
        format: FORMAT,
        device,
    }
}

#[cfg(target_os = "macos")]
fn host(capabilities: &DeviceCapabilities, queue: &wgpu::Queue, extent: [u32; 2]) -> support::Host {
    let mut scene = SyntheticScene::grid(1).expect("test scene");
    scene.view = scene.view.with_reconstruction(true);
    let mut host = pollster::block_on(support::host_with_config(
        capabilities,
        queue,
        scene,
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
fn submit(host: &mut support::Host, view: &wgpu::TextureView, extent: [u32; 2]) {
    let report = host
        .drive(Duration::ZERO, Some(&SurfaceOutput { view, extent }))
        .expect("transition fixture");
    assert!(report.submitted && report.surface_presented);
}

#[cfg(target_os = "macos")]
fn complete(host: &mut support::Host, device: &wgpu::Device) -> FrameSample {
    thrum_test_support::wait::until("completed Metal frame", Duration::from_secs(5), || {
        device
            .poll(wgpu::PollType::Poll)
            .expect("transition fixture");
        host.poll_completed().expect("transition fixture")
    })
}

#[cfg(target_os = "macos")]
type ExtentJob = fn(&AtomicBool) -> Result<Option<PreparedExtent>, FrameError>;

#[cfg(target_os = "macos")]
fn unexpected_build() -> Result<ExtentJob, FrameError> {
    panic!("request was not yet eligible to build");
}

#[cfg(target_os = "macos")]
fn prepare(
    host: &support::Host,
    extent: [u32; 2],
    dispatch: Dispatch,
) -> impl FnOnce(&AtomicBool) -> Result<Option<PreparedExtent>, FrameError> + Send + use<> {
    let inputs = host
        .extent_prepare_inputs(extent)
        .expect("transition fixture");
    move |cancel| tracing::dispatcher::with_default(&dispatch, || inputs.build(cancel))
}

#[cfg(target_os = "macos")]
fn captured(host: &mut support::Host, device: &wgpu::Device) -> SubmittedImage {
    thrum_test_support::wait::until(
        "capture drained through the switch",
        Duration::from_secs(5),
        || {
            device.poll(wgpu::PollType::Poll).expect("capture progress");
            host.try_submitted_image().expect("capture mapping")
        },
    )
}

#[cfg(target_os = "macos")]
fn first_frame_matches_reset(actual: &SubmittedImage, reset: &SubmittedImage) -> bool {
    actual.extent == reset.extent
        && actual.sample.history_identity[1] == reset.sample.history_identity[1]
        && actual.rgba == reset.rgba
}

#[cfg(target_os = "macos")]
fn independent_reset(
    cap: &DeviceCapabilities,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    output: &wgpu::TextureView,
    extent: [u32; 2],
    jitter_phase: usize,
) -> SubmittedImage {
    let mut independent = host(cap, queue, extent);
    for _ in 0..jitter_phase {
        submit(&mut independent, output, extent);
        complete(&mut independent, device);
    }
    independent
        .enable_submitted_capture_with_limits(1, CaptureLimits::default())
        .expect("independent reset tap");
    independent.invalidate_history();
    submit(&mut independent, output, extent);
    complete(&mut independent, device);
    captured(&mut independent, device)
}

thrum_test_support::macos_gpu_test! {
fn retaining_history_across_a_backing_scale_switch_fails_the_first_frame_comparison() {
    let _metal = METAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(thrum_test_support::Opened { gpu, device, queue, .. }) = thrum_test_support::open(
        "kept extent history control",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let output = view(&device, OLD);
    let reset = independent_reset(&cap, &device, &queue, &output, OLD, 1);

    // Hold pixel dimensions fixed as the backing scale changes, so the
    // output-size validity key cannot mask a missing transition reset.
    for keep_history in [false, true] {
        let mut host = host(&cap, &queue, OLD);
        submit(&mut host, &output, OLD);
        let old_frame = complete(&mut host, &device);
        host.enable_submitted_capture_with_limits(7, CaptureLimits::default())
            .expect("switch tap");
        let old = request(OLD, 1);
        let scale_changed = ExtentRequest {
            backing_scale: 1_f64.to_bits(),
            ..old
        };
        let mut transition = ExtentTransition::new(old);
        let candidate = thrum_test_support::wait::until(
            "scale change built immediately",
            Duration::from_secs(5),
            || {
                transition
                    .advance(scale_changed, old, Duration::ZERO, || {
                        Ok(prepare(&host, OLD, Dispatch::none()))
                    })
                    .expect("scale build");
                transition.ready()
            },
        );
        assert_eq!(
            transition
                .surface
                .take(false, transition.busy(), &OLD, &OLD, |_| Ok::<_, &str>(())),
            SurfaceSwitchOutcome::Taken
        );
        if keep_history {
            assert!(host.publish_extent_keeping_history_for_test(candidate));
        } else {
            assert!(host.publish_extent(candidate));
        }
        submit(&mut host, &output, OLD);
        complete(&mut host, &device);
        let first_new = captured(&mut host, &device);
        assert_eq!(
            first_new.sample.submission_order,
            old_frame.submission_order + 1
        );
        if keep_history {
            assert_eq!(
                first_new.sample.history_identity[1],
                old_frame.history_identity[0]
            );
            assert!(!first_frame_matches_reset(&first_new, &reset));
            assert_ne!(
                first_new.rgba, reset.rgba,
                "old reconstruction changed the first image"
            );
            eprintln!(
                "kept history: first new predecessor {}, reset predecessor {}, pixels equal {}",
                first_new.sample.history_identity[1],
                reset.sample.history_identity[1],
                first_new.rgba == reset.rgba,
            );
        } else {
            assert!(first_frame_matches_reset(&first_new, &reset));
        }
    }
}
}

thrum_test_support::macos_gpu_test! {
fn a_detached_extent_preserves_current_capture_admission() {
    let _metal = METAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(thrum_test_support::Opened { gpu, device, queue, .. }) = thrum_test_support::open(
        "extent capture admission",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let old_output = view(&device, OLD);
    let new_output = view(&device, NEW);
    let build = |host: &support::Host| {
        let job = prepare(host, NEW, Dispatch::none());
        std::thread::spawn(move || {
            job(&AtomicBool::new(false))
                .expect("capture extent")
                .expect("candidate")
        })
        .join()
        .expect("preparation thread")
    };

    let mut stopped = host(&cap, &queue, OLD);
    stopped
        .enable_submitted_capture_with_limits(7, CaptureLimits::default())
        .expect("tap");
    submit(&mut stopped, &old_output, OLD);
    let old = complete(&mut stopped, &device);
    let candidate = build(&stopped);
    stopped.stop_submitted_capture();
    assert!(stopped.publish_extent(candidate));
    submit(&mut stopped, &new_output, NEW);
    complete(&mut stopped, &device);
    assert_eq!(stopped.submitted_capture_usage(), Some((1, 0)));
    let image = captured(&mut stopped, &device);
    assert_eq!(
        (image.session, image.extent, image.sample.submission_order),
        (7, OLD, old.submission_order)
    );
    stopped.recycle_submitted_image(image);
    assert_eq!(stopped.submitted_capture_usage(), Some((0, 0)));
    assert!(
        stopped
            .try_submitted_image()
            .expect("stopped tap drained")
            .is_none()
    );

    let mut replaced = host(&cap, &queue, OLD);
    replaced
        .enable_submitted_capture_with_limits(7, CaptureLimits::default())
        .expect("tap");
    let obsolete = build(&replaced);
    replaced.disable_submitted_capture();
    assert!(!replaced.admits_extent(&obsolete));
    let disabled = build(&replaced);
    replaced
        .enable_submitted_capture_with_limits(7, CaptureLimits::default())
        .expect("replacement with reused session number");
    assert!(!replaced.admits_extent(&disabled));
    assert!(!replaced.publish_extent(obsolete));
    let current = build(&replaced);
    assert!(replaced.publish_extent(current));
    submit(&mut replaced, &new_output, NEW);
    complete(&mut replaced, &device);
    assert_eq!(captured(&mut replaced, &device).extent, NEW);
}
}

thrum_test_support::macos_gpu_test! {
fn mac_host_holds_old_frames_and_only_publishes_latest_generation() {
    let _metal = METAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(thrum_test_support::Opened { gpu, device, queue, .. }) = thrum_test_support::open(
        "extent generation transition",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let mut host = host(&cap, &queue, OLD);
    host.enable_submitted_capture_with_limits(
        7,
        CaptureLimits {
            // The proof drains all five old and new images after the switch.
            staging_slots: 5,
            ..CaptureLimits::default()
        },
    )
    .expect("transition fixture");
    let old_output = view(&device, OLD);
    let new_output = view(&device, NEW);
    let reset_image = independent_reset(&cap, &device, &queue, &new_output, NEW, 3);
    let single_view_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("one-view creation control"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let creations = Creations::default();
    let dispatch = creations.dispatch();
    let _render_events = tracing::dispatcher::set_default(&dispatch);
    let render_thread = std::thread::current().id();
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
    let (built, observed_build) = mpsc::channel();
    let first_build = prepare(&host, STALE, dispatch.clone());
    transition
        .advance(stale, old, ms(100), || {
            Ok(move |cancel: &AtomicBool| {
                // Hold a finished result on the preparation thread until it is superseded.
                let ready = first_build(cancel)?.expect("superseded candidate resources");
                built
                    .send(std::thread::current().id())
                    .expect("notify completed preparation");
                blocked
                    .recv_timeout(Duration::from_secs(10))
                    .expect("transition fixture");
                assert!(cancel.load(Ordering::Acquire));
                Ok(Some(ready))
            })
        })
        .expect("transition fixture");

    submit(&mut host, &old_output, OLD);
    let first = complete(&mut host, &device);
    let worker_thread = observed_build
        .recv_timeout(Duration::from_secs(5))
        .expect("observe completed preparation");
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

    let candidate = thrum_test_support::wait::until(
        "latest build ready",
        Duration::from_secs(10),
        || {
            transition
                .advance(new, old, ms(202), || {
                    Ok(prepare(&host, NEW, dispatch.clone()))
                })
                .expect("transition fixture");
            transition.ready()
        },
    );
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
    assert_eq!(transition.surface.record().configure_calls, 1);
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
    assert_eq!(
        switched.history_identity[1],
        reset_image.sample.history_identity[1]
    );
    submit(&mut host, &new_output, NEW);
    let resumed = complete(&mut host, &device);
    assert_eq!(resumed.history_identity[1], switched.history_identity[0]);
    assert_ne!(
        resumed.history_identity[1],
        reset_image.sample.history_identity[1]
    );

    let switched_submission = switched.submission_order;
    let expected = [first, held, last_old, switched, resumed];
    for expected in expected {
        let image = captured(&mut host, &device);
        if image.sample.submission_order == switched_submission {
            assert!(
                first_frame_matches_reset(&image, &reset_image),
                "first new image differs from the independent reset"
            );
        }
        assert_eq!(image.session, 7);
        assert_eq!(image.sample.submission_order, expected.submission_order);
        assert_eq!(image.extent, expected.render_extent);
        assert_eq!(image.sample.history_identity, expected.history_identity);
        host.recycle_submitted_image(image);
    }
    assert_eq!(host.submitted_capture_omissions(), 0);

    assert_ne!(worker_thread, render_thread);
    let worker = creations.on(worker_thread);
    for kind in [
        "Device::create_texture",
        "Texture::create_view",
        "Device::create_bind_group",
    ] {
        assert!(
            worker.contains(&kind),
            "missing worker event {kind}: {worker:?}"
        );
    }
    // The surface double lends pre-existing drawable views, so it needs no
    // exception for the window host's ordinary drawable view creation.
    let render_created_nothing = || {
        let events = creations.on(render_thread);
        if events.is_empty() {
            Ok(())
        } else {
            Err(events)
        }
    };
    assert_eq!(render_created_nothing(), Ok(()));
    drop(single_view_texture.create_view(&wgpu::TextureViewDescriptor::default()));
    assert_eq!(render_created_nothing(), Err(vec!["Texture::create_view"]));
    let control = prepare(&host, STALE, dispatch.clone())(&AtomicBool::new(false))
        .expect("transition fixture");
    let wrong_thread = render_created_nothing().expect_err("synchronous build escaped detection");
    assert!(wrong_thread.contains(&"Device::create_texture"));
    assert!(wrong_thread.contains(&"Device::create_bind_group"));
    eprintln!(
        "preparation thread created {} objects; render owner created zero until controls; single view detected; synchronous build detected {} objects",
        worker.len(),
        wrong_thread.len() - 1,
    );
    drop(control);
}
}

thrum_test_support::macos_gpu_test! {
fn failed_and_lost_builds_never_take_the_surface() {
    let _metal = METAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let restriction = thrum_test_support::Restriction::default();
    let Some(thrum_test_support::Opened { gpu, device, queue, .. }) = thrum_test_support::open(
        "failed extent generation",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &restriction,
    ) else {
        return;
    };
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
            Ok(|_: &AtomicBool| Err(FrameError::gpu("forced allocation failure", thrum_render::error::RenderError::TargetExtent)))
        })
        .expect("transition fixture");
    let failure = thrum_test_support::wait::until(
        "forced build failure",
        Duration::from_secs(5),
        || transition.advance(stale, old, ms(101), unexpected_build).err(),
    );
    assert!(failure.to_string().contains("forced allocation failure"));
    transition
        .advance(stale, old, ms(900), unexpected_build)
        .expect("transition fixture");
    assert!(transition.ready().is_none());
    assert_eq!(transition.surface.record().configure_calls, 0);
    submit(&mut host, &old_output, OLD);
    assert_eq!(complete(&mut host, &device).render_extent, OLD);

    let inputs = host.device_prepare_inputs().expect("transition fixture");
    let job = prepare(&host, NEW, Dispatch::none());
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
    let ready = thrum_test_support::wait::until(
        "recovered build ready",
        Duration::from_secs(5),
        || {
            transition
                .advance(new, old, ms(1300), || {
                    Ok(prepare(&host, NEW, Dispatch::none()))
                })
                .expect("transition fixture");
            transition.ready()
        },
    );
    assert_eq!(transition.surface.record().configure_calls, 0);
    assert!(host.admits_extent(&ready));
    assert!(host.publish_extent(ready));
    let new_output = view(&device, NEW);
    submit(&mut host, &new_output, NEW);
    assert_eq!(complete(&mut host, &device).render_extent, NEW);
}
}
