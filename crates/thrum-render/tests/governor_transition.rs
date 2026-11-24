//! The installed Metal generation keeps presenting after an asynchronous failure or cancellation.

mod support;

use std::{
    sync::{atomic::AtomicBool, mpsc},
    time::Duration,
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    error::RenderError,
    governor::{DisplayRecord, ThermalState},
    present::SurfaceProfile,
    runtime::{
        ExtentRequest, ExtentTransition, FrameConfig, FrameError, FrameSample, PreparedExtent,
        SurfaceOutput,
    },
    synthetic::{FrameLighting, RenderScale, SyntheticLight, SyntheticScene},
};

const OLD: [u32; 2] = [64, 64];
const NEW: [u32; 2] = [96, 64];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DISPLAY: DisplayRecord = DisplayRecord {
    interval: Duration::from_millis(50),
    thermal: ThermalState::Nominal,
};
type Job = fn(&AtomicBool) -> Result<Option<PreparedExtent>, FrameError>;

fn unexpected_build() -> Result<Job, FrameError> {
    panic!("the unchanged request must not rebuild");
}

fn present(
    host: &mut support::Host,
    output: &wgpu::TextureView,
    held: bool,
    now_ms: u64,
    tick: u64,
) -> FrameSample {
    let now = Duration::from_millis(now_ms);
    host.governor_display(DISPLAY, held);
    host.governor_display_tick(now, tick);
    let report = host
        .drive(
            now,
            Some(&SurfaceOutput {
                view: output,
                extent: OLD,
            }),
        )
        .expect("render retained generation");
    assert!(report.submitted && report.surface_presented);
    host.governor_present_called();
    let sample =
        thrum_test_support::wait::until("retained Metal output", Duration::from_secs(5), || {
            host.poll_completed().expect("poll retained generation")
        });
    assert_eq!(
        sample
            .governor
            .as_ref()
            .expect("governor state")
            .mark
            .held(),
        held
    );
    sample
}

fn prove(
    cap: &DeviceCapabilities,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cancelled: bool,
    stuck_hold_control: bool,
) -> bool {
    let mut scene = SyntheticScene::grid(1).expect("small scene");
    scene.view = scene.view.with_reconstruction(true);
    let mut host = pollster::block_on(support::host_with_config(
        cap,
        queue,
        scene,
        FrameLighting::Diagnostic(SyntheticLight::default()),
        FrameConfig {
            render_extent: OLD,
            surface: Some(SurfaceProfile {
                format: FORMAT,
                extent: OLD,
            }),
        },
    ));
    let output = support::drawable(device, FORMAT, OLD);
    assert!(host.enable_governor(DISPLAY));
    let old = ExtentRequest {
        extent: OLD,
        format: FORMAT,
        backing_scale: 2_f64.to_bits(),
        device: 1,
    };
    let new = ExtentRequest { extent: NEW, ..old };
    let mut transition = ExtentTransition::new(old);
    let first = present(&mut host, &output, false, 0, 1);
    let (started, worker_started) = mpsc::channel();
    let (release, worker_release) = mpsc::channel();
    let inputs = host
        .extent_prepare_inputs(NEW)
        .expect("resize candidate inputs");
    transition.observe(new, old, Duration::ZERO);
    transition
        .advance(new, old, Duration::from_millis(100), || {
            Ok(move |cancel: &AtomicBool| {
                started.send(()).expect("test owns the build gate");
                worker_release
                    .recv_timeout(Duration::from_secs(4))
                    .expect("test releases preparation");
                if cancelled {
                    inputs.build(cancel)
                } else {
                    // Same allocation refusal used by the generation transition's failure arm.
                    Err(FrameError::gpu(
                        "forced allocation failure",
                        RenderError::TargetExtent,
                    ))
                }
            })
        })
        .expect("start detached resize");
    worker_started
        .recv_timeout(Duration::from_secs(4))
        .expect("resize started");
    let held = present(&mut host, &output, transition.holds_governor(), 110, 5);
    assert_eq!(
        held.render_extent, OLD,
        "misses during the build leave quality alone"
    );
    let request = if cancelled { old } else { new };
    if cancelled {
        transition.observe(request, old, Duration::from_millis(120));
    }
    release.send(()).expect("resize released");
    let failed = thrum_test_support::wait::until(
        "cancelled or failed resize leaves no preparation",
        Duration::from_secs(5),
        || {
            let result =
                transition.advance(request, old, Duration::from_millis(120), unexpected_build);
            (!transition.holds_governor()).then_some(result.is_err())
        },
    );
    assert_eq!(failed, !cancelled);
    assert!(transition.ready().is_none());
    assert_eq!(transition.surface.record().generations_taken, 0);
    let hold = stuck_hold_control || transition.holds_governor();
    let resumed = present(&mut host, &output, hold, 130, 6);
    assert_eq!(resumed.render_extent, OLD);
    // Force render-side load through the display opportunity feed. The drawable
    // still presents at the next opportunity, but three callbacks were missed.
    let loaded = present(&mut host, &output, hold, 330, 10);
    let dropped = loaded.governor.expect("loaded frame").state.scale == RenderScale::Percent85;
    let mut recovered = false;
    for frame in 1..=16 {
        transition
            .advance(
                request,
                old,
                Duration::from_millis(330 + frame * 100),
                unexpected_build,
            )
            .expect("unchanged request retains the old generation");
        let sample = present(&mut host, &output, hold, 330 + frame * 100, 10 + frame);
        assert_eq!(sample.tick, first.tick);
        recovered |= sample.governor.expect("governor frame").state.scale == RenderScale::Native;
    }
    dropped && recovered
}

thrum_test_support::macos_gpu_test! {
fn failure_and_cancel_reopen_governor_on_the_unchanged_request() {
    let Some(thrum_test_support::Opened { gpu, device, queue, .. }) =
        thrum_test_support::open("governor across extent preparation", Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable, &thrum_test_support::Restriction::default())
    else { return; };
    let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    for cancelled in [false, true] {
        assert!(prove(&cap, &device, &queue, cancelled, false));
        assert!(!prove(&cap, &device, &queue, cancelled, true));
    }
}
}
