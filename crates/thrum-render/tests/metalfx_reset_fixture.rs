//! Exercise reset decisions through the host that owns the native segment.
use crate::support;

use std::{io::Write, time::Duration};
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{CaptureLimits, FrameConfig, SurfaceOutput},
    synthetic::{
        CaptureSource, FrameLighting, MetalFxProbe, SyntheticLight, SyntheticScene, SyntheticView,
    },
};

const EXTENT: [u32; 2] = [64, 64];
const PROFILE: SurfaceProfile = SurfaceProfile {
    format: wgpu::TextureFormat::Rgba8Unorm,
    extent: EXTENT,
};

fn view(x: f32) -> SyntheticView {
    SyntheticView::perspective(
        [x, 0.4, 2.0],
        [x, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        0.8,
        1.0,
        0.1,
        20.0,
    )
    .expect("fixture camera")
    .with_metalfx()
}

fn host(caps: &DeviceCapabilities, queue: &wgpu::Queue, probe: MetalFxProbe) -> support::Host {
    let mut scene = SyntheticScene::grid(5).expect("fixture");
    scene.sun_shadows = false;
    scene.view = view(0.0).with_metalfx_probe(probe);
    pollster::block_on(support::host_with_config_exact(
        caps,
        queue,
        scene,
        FrameLighting::Diagnostic(SyntheticLight::default()),
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(PROFILE),
        },
    ))
}

fn capture(
    host: &mut support::Host,
    device: &wgpu::Device,
    surface: &SurfaceOutput<'_>,
) -> Vec<u8> {
    let next = host.last_submission().map_or(0, |order| order + 1);
    host.enable_submitted_capture_from(next, CaptureLimits::default(), CaptureSource::NativeOutput)
        .expect("native submitted-image tap");
    let frame = host
        .drive(Duration::ZERO, Some(surface))
        .expect("fixture submitted");
    assert!(frame.surface_presented);
    let pixels =
        thrum_test_support::wait::until("submitted native image", Duration::from_secs(10), || {
            device.poll(wgpu::PollType::Poll).expect("poll fixture");
            drop(host.poll_completed().expect("fixture completion"));
            let image = host.try_submitted_image().expect("native image")?;
            let pixels = image.rgba.clone();
            host.recycle_submitted_image(image);
            Some(pixels)
        });
    host.disable_submitted_capture();
    while host.pending_frames() != 0 {
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("drain fixture");
        drop(host.poll_completed().expect("drain completion"));
    }
    pixels
}

fn warm(host: &mut support::Host, device: &wgpu::Device, surface: &SurfaceOutput<'_>) {
    for _ in 0..4 {
        capture(host, device, surface);
    }
}

#[derive(Clone, Copy, Debug)]
enum Refusal {
    Acquisition,
    Submission,
    ExplicitReset,
}

fn arm(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    surface: &SurfaceOutput<'_>,
    probe: MetalFxProbe,
    refusal: Refusal,
) -> (Vec<u8>, thrum_render::synthetic::NativeProviderReport) {
    let device = caps.site();
    let mut host = host(caps, queue, probe);
    warm(&mut host, device, surface);
    host.continuous_source_mut().view = view(0.08);
    match refusal {
        Refusal::Acquisition => {
            let report = host
                .advance(Duration::ZERO)
                .expect("advanced fixture")
                .encode_scene()
                .expect("encoded fixture")
                .submit(None)
                .expect("no acquired texture");
            assert!(report.dropped_view && !report.submitted);
        }
        Refusal::Submission => {
            let error = host
                .advance(Duration::ZERO)
                .expect("advanced fixture")
                .encode_scene()
                .expect("encoded fixture")
                .refuse_submission_for_probe(Some(surface))
                .expect_err("queue admission refused");
            assert!(matches!(
                error,
                thrum_render::runtime::FrameError::InvalidState("injected submission refusal")
            ));
        }
        Refusal::ExplicitReset => host.invalidate_history(),
    }
    let pixels = capture(&mut host, device, surface);
    (
        pixels,
        host.native_provider_report().expect("segment counters"),
    )
}

pub(super) fn acquisition_submission_and_device_replacement_reset_native_history() {
    let Some(opened) = thrum_test_support::open(
        "MetalFX lifecycle",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Native,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let apple = opened
        .capabilities
        .clone()
        .with_recipe_override("apple-tile")
        .expect("Apple tile recipe");
    let portable = opened
        .capabilities
        .with_recipe_override("portable")
        .expect("portable recipe");
    let texture = support::drawable(&opened.device, PROFILE.format, EXTENT);
    let surface = SurfaceOutput {
        view: &texture,
        extent: EXTENT,
    };
    let (reset, _) = arm(
        &apple,
        &opened.queue,
        &surface,
        MetalFxProbe::None,
        Refusal::ExplicitReset,
    );
    for refusal in [Refusal::Acquisition, Refusal::Submission] {
        let (recovered, native) = arm(&apple, &opened.queue, &surface, MetalFxProbe::None, refusal);
        assert_eq!(recovered, reset, "{refusal:?} did not reset MetalFX");
        assert_eq!(
            (
                native.discarded,
                native.resets,
                native.discards_kept_history
            ),
            (1, 2, 0)
        );
        let (stale, _) = arm(
            &apple,
            &opened.queue,
            &surface,
            MetalFxProbe::SkipReset,
            refusal,
        );
        assert_ne!(
            stale, reset,
            "{refusal:?} did not detect a skipped MetalFX reset"
        );
        let (_, twin) = arm(
            &portable,
            &opened.queue,
            &surface,
            MetalFxProbe::None,
            refusal,
        );
        assert_eq!(twin.discarded, 1);
        let acquisition = matches!(refusal, Refusal::Acquisition);
        assert_eq!(twin.discards_kept_history, u64::from(acquisition));
        assert_eq!(twin.resets, if acquisition { 1 } else { 2 });
        writeln!(
            std::io::stdout().lock(),
            "{refusal:?}: native {native:?}; portable {twin:?}"
        )
        .expect("write reset proof");
    }
    let mut replaced = host(&apple, &opened.queue, MetalFxProbe::None);
    warm(&mut replaced, &opened.device, &surface);
    replaced
        .advance(Duration::ZERO)
        .expect("advanced fixture")
        .encode_scene()
        .expect("encoded fixture")
        .abandon_lost_device();
    let retired = replaced.native_provider_report().expect("old provider");
    assert_eq!(retired.discarded, 1);
    let inputs = replaced.device_prepare_inputs().expect("generation inputs");
    let (device, queue, caps) = opened
        .gpu
        .fresh_adapter()
        .device_with_hardware(
            thrum_test_support::Profile::Native,
            &thrum_test_support::Restriction::default(),
            2,
            &[],
        )
        .expect("replacement device");
    let caps = caps
        .with_recipe_override("apple-tile")
        .expect("Apple tile recipe");
    let mut generation = pollster::block_on(inputs.prepare(&caps, &queue, Some(PROFILE)))
        .expect("prepared replacement");
    replaced
        .publish_device(&mut generation)
        .expect("publish replacement");
    assert_eq!(
        replaced
            .native_provider_report()
            .expect("native provider")
            .resets,
        0
    );
    let texture = support::drawable(&device, PROFILE.format, EXTENT);
    let surface = SurfaceOutput {
        view: &texture,
        extent: EXTENT,
    };
    let post = capture(&mut replaced, &device, &surface);
    let mut fresh = host(&caps, &queue, MetalFxProbe::None);
    warm(&mut fresh, &device, &surface);
    fresh.invalidate_history();
    let reset = capture(&mut fresh, &device, &surface);
    assert_eq!(
        post, reset,
        "first replacement frame differs from independent reset"
    );
    let next = replaced
        .native_provider_report()
        .expect("replacement provider");
    assert_eq!((next.resets, next.submitted), (1, 1));
    writeln!(
        std::io::stdout().lock(),
        "device generation 1 {retired:?}; generation 2 {next:?}"
    )
    .expect("write generation proof");
}
