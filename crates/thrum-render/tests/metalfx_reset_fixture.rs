//! Exercise reset decisions through the host that owns the native segment.
use crate::support;

use std::{io::Write, time::Duration};
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{CaptureLimits, FrameConfig, SurfaceOutput},
    synthetic::{
        CaptureSource, FrameLighting, MetalFxProbe, NativeProviderReport, SyntheticLight,
        SyntheticScene, SyntheticView, TwinReason,
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
) -> (Vec<u8>, NativeProviderReport) {
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

fn open_metal() -> Option<thrum_test_support::Opened> {
    thrum_test_support::open(
        "MetalFX lifecycle",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Native,
        &thrum_test_support::Restriction::default(),
    )
}

pub(super) fn acquisition_and_submission_reset_native_history() {
    let Some(opened) = open_metal() else {
        return;
    };
    let apple = opened
        .capabilities
        .clone()
        .with_recipe_override("apple-tile")
        .expect("Apple tile recipe");
    let portable = opened
        .capabilities
        .clone()
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
        assert!(native.twin.is_none());
        assert_eq!(
            twin.twin.expect("portable twin").reason,
            TwinReason::PortableRecipe
        );
        writeln!(
            std::io::stdout().lock(),
            "{refusal:?}: native {native:?}; portable {twin:?}"
        )
        .expect("write reset proof");
    }
}

fn replace_device(
    opened: &thrum_test_support::Opened,
    recipe: &str,
    probe: MetalFxProbe,
    replacement: &DeviceCapabilities,
    replacement_queue: &wgpu::Queue,
    replacement_surface: &SurfaceOutput<'_>,
) -> (Vec<u8>, Option<bool>) {
    let original = opened
        .capabilities
        .clone()
        .with_recipe_override(recipe)
        .expect("original recipe");
    let replacement = replacement
        .clone()
        .with_recipe_override(recipe)
        .expect("replacement recipe");
    let texture = support::drawable(&opened.device, PROFILE.format, EXTENT);
    let surface = SurfaceOutput {
        view: &texture,
        extent: EXTENT,
    };
    let mut replaced = host(&original, &opened.queue, MetalFxProbe::None);
    warm(&mut replaced, &opened.device, &surface);
    let before_loss = replaced.last_submission();
    replaced
        .advance(Duration::ZERO)
        .expect("advanced fixture")
        .encode_scene()
        .expect("encoded fixture")
        .abandon_lost_device();
    assert_eq!(replaced.last_submission(), before_loss);
    let retired = replaced.native_provider_report().expect("old provider");
    assert_eq!(
        (retired.encoded, retired.submitted, retired.discarded),
        (5, 4, 1)
    );
    assert_eq!(retired.discards_kept_history, 0);
    // Select the control only for the replacement scaler, after warming and
    // abandoning the old generation through the ordinary host path.
    replaced.continuous_source_mut().view = view(0.0).with_metalfx_probe(probe);
    let inputs = replaced.device_prepare_inputs().expect("generation inputs");
    let mut generation =
        pollster::block_on(inputs.prepare(&replacement, replacement_queue, Some(PROFILE)))
            .expect("prepared replacement");
    replaced
        .publish_device(&mut generation)
        .expect("publish replacement");
    assert_eq!(replaced.metalfx_last_encoded_reset(), None);
    let post = capture(&mut replaced, replacement.site(), replacement_surface);
    let first_reset = replaced.metalfx_last_encoded_reset();
    let next = replaced
        .native_provider_report()
        .expect("replacement provider");
    assert_eq!(
        (next.encoded, next.resets, next.submitted, next.completed),
        (1, 1, 1, 1)
    );
    let twin = if recipe == "portable" {
        Some(TwinReason::PortableRecipe)
    } else {
        None
    };
    assert_eq!(retired.twin.map(|selection| selection.reason), twin);
    assert_eq!(next.twin.map(|selection| selection.reason), twin);
    writeln!(
        std::io::stdout().lock(),
        "{recipe} {probe:?}: retired generation {} {retired:?}; replacement generation {} {next:?}; first scaler reset {first_reset:?}",
        original.generation(),
        replacement.generation()
    )
    .expect("write generation proof");
    (post, first_reset)
}

pub(super) fn device_replacement_resets_native_history() {
    let Some(opened) = open_metal() else {
        return;
    };
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
    let texture = support::drawable(&device, PROFILE.format, EXTENT);
    let surface = SurfaceOutput {
        view: &texture,
        extent: EXTENT,
    };
    let (post, first_reset) = replace_device(
        &opened,
        "apple-tile",
        MetalFxProbe::None,
        &caps,
        &queue,
        &surface,
    );
    assert_eq!(
        first_reset,
        Some(true),
        "replacement scaler needs a first-encode reset"
    );
    let (_, skipped_reset) = replace_device(
        &opened,
        "apple-tile",
        MetalFxProbe::SkipFirstReset,
        &caps,
        &queue,
        &surface,
    );
    // A pristine scaler may render identical pixels with either flag. The
    // actual SDK argument makes the same reset requirement reject the control.
    assert_eq!(
        skipped_reset,
        Some(false),
        "control must omit the first reset"
    );
    let (_, portable_reset) = replace_device(
        &opened,
        "portable",
        MetalFxProbe::None,
        &caps,
        &queue,
        &surface,
    );
    assert_eq!(portable_reset, None, "portable replacement must use TAAU");

    let apple = caps
        .with_recipe_override("apple-tile")
        .expect("Apple tile recipe");
    let mut fresh = host(&apple, &queue, MetalFxProbe::None);
    warm(&mut fresh, &device, &surface);
    fresh.invalidate_history();
    let reset = capture(&mut fresh, &device, &surface);
    assert_eq!(
        post, reset,
        "first replacement frame differs from independent reset"
    );
}
