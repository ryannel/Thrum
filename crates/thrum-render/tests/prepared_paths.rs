//! Mac Metal preparation, with platform facts forced at their renderer boundary.
#![cfg(target_os = "macos")]
mod support;

use std::{io::Write, sync::atomic::AtomicBool, time::Duration};
use thrum_render::{
    governor::{DisplayRecord, ThermalState},
    present::SurfaceProfile,
    runtime::{CreationPhase, CreationReport, FrameConfig, FrameSample, SurfaceOutput},
    synthetic::{FrameLighting, Reconstruction, SyntheticLight, SyntheticScene},
};
use thrum_test_support::{Opened, Profile, Restriction};

const EXTENT: [u32; 2] = [64, 64];
const RESIZED: [u32; 2] = [96, 48];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

fn profile(extent: [u32; 2]) -> SurfaceProfile {
    SurfaceProfile {
        format: FORMAT,
        extent,
    }
}

fn lighting() -> FrameLighting {
    FrameLighting::Diagnostic(SyntheticLight::default())
}

fn open(name: &str) -> Opened {
    let mut gpu = thrum_test_support::open(
        name,
        Some(wgpu::Backend::Metal),
        Profile::Native,
        &Restriction::default(),
    )
    .expect("prepared paths require actual Metal");
    gpu.capabilities = gpu
        .capabilities
        .with_recipe_override("apple-tile")
        .expect("Apple tile");
    gpu
}

fn ready(gpu: &Opened) -> support::Host {
    let mut scene = SyntheticScene::grid(1).expect("fixture");
    scene.sun_shadows = false;
    scene.view = scene.view.with_metalfx();
    let host = pollster::block_on(support::host_with_config_exact(
        &gpu.capabilities,
        &gpu.queue,
        scene,
        lighting(),
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(profile(EXTENT)),
        },
    ));
    assert_eq!(host.reconstruction(), Reconstruction::MetalFx);
    let report = host.creation_report();
    assert!(report.ready && report.frames == 0);
    assert!(report.pipeline_events(CreationPhase::Build) > 0, "{report}");
    assert_eq!(report.native_events(CreationPhase::Build), 1, "{report}");
    host
}

fn draw(host: &mut support::Host, view: &wgpu::TextureView, extent: [u32; 2]) -> FrameSample {
    let frame = host
        .drive(Duration::ZERO, Some(&SurfaceOutput { view, extent }))
        .expect("draw");
    assert!(frame.submitted && frame.surface_presented, "{frame:?}");
    thrum_test_support::wait::until("Metal draw", Duration::from_secs(10), || {
        host.poll_completed().expect("Metal completion")
    })
}

fn receipt(name: &str, before: &CreationReport, after: &CreationReport) {
    writeln!(
        std::io::stdout().lock(),
        "{name}\n  before {before}\n  after  {after}"
    )
    .expect("receipt");
}

fn unchanged_creations(before: &CreationReport, after: &CreationReport) {
    assert_eq!(before.pipelines, after.pipelines, "{after}");
    assert_eq!(before.rows, after.rows, "{after}");
    assert_eq!(before.native_creations, after.native_creations, "{after}");
    assert_eq!(after.violations, 0, "{after}");
}

#[test]
fn replaced_drawable_reuses_ready_scene_and_history() {
    let gpu = open("surface replacement");
    // AppKit's same-extent lease replacement supplies a new drawable. The
    // renderer consumes the new binding at encode time.
    let old = support::drawable(&gpu.device, FORMAT, EXTENT);
    let replacement = support::drawable(&gpu.device, FORMAT, EXTENT);
    let mut host = ready(&gpu);
    let first = draw(&mut host, &old, EXTENT);
    let before = host.creation_checkpoint();
    let next = draw(&mut host, &replacement, EXTENT);
    let after = host.creation_checkpoint();
    receipt("replaced drawable", &before, &after);
    assert_ne!(old, replacement);
    assert_eq!(next.render_extent, first.render_extent);
    assert_eq!(next.history_identity[1], first.history_identity[0]);
    assert_eq!(after.generation, before.generation);
    unchanged_creations(&before, &after);
}

#[test]
fn forced_refresh_changes_the_budget_and_keeps_render_quality() {
    let gpu = open("refresh change");
    let output = support::drawable(&gpu.device, FORMAT, EXTENT);
    let mut host = ready(&gpu);
    let display = DisplayRecord {
        interval: Duration::from_millis(500),
        thermal: ThermalState::Nominal,
    };
    assert!(host.enable_governor(display));
    let previous = draw(&mut host, &output, EXTENT);
    let before = host.creation_checkpoint();
    let changed = DisplayRecord {
        interval: Duration::from_millis(250),
        ..display
    };
    host.governor_display(changed, false);
    let next = draw(&mut host, &output, EXTENT);
    let old = previous.governor.unwrap();
    let new = next.governor.unwrap();
    assert!(new.budget < old.budget);
    assert_eq!(new.display.interval, changed.interval);
    assert_eq!(new.state.scale, old.state.scale);
    assert_eq!(new.state.tier, old.state.tier);
    assert_eq!(new.knobs, old.knobs);
    assert_eq!(host.display_headroom().to_bits(), 1.0_f32.to_bits());
    assert_eq!(next.render_extent, previous.render_extent);
    assert_eq!(next.history_identity[1], previous.history_identity[0]);
    let after = host.creation_checkpoint();
    receipt("forced refresh", &before, &after);
    unchanged_creations(&before, &after);
}

#[test]
fn forced_display_move_updates_only_headroom() {
    let gpu = open("display move");
    let output = support::drawable(&gpu.device, FORMAT, EXTENT);
    let mut host = ready(&gpu);
    assert!(host.is_edr());
    let display = DisplayRecord {
        interval: Duration::from_millis(500),
        thermal: ThermalState::Nominal,
    };
    assert!(host.enable_governor(display));
    let previous = draw(&mut host, &output, EXTENT);
    let before = host.creation_checkpoint();
    host.set_display_headroom(2.0);
    let next = draw(&mut host, &output, EXTENT);
    assert_eq!(host.display_headroom().to_bits(), 2.0_f32.to_bits());
    assert_eq!(next.render_extent, previous.render_extent);
    assert_eq!(next.history_identity[1], previous.history_identity[0]);
    let old = previous.governor.unwrap();
    let new = next.governor.unwrap();
    assert_eq!(new.budget, old.budget);
    assert_eq!(new.state.scale, old.state.scale);
    assert_eq!(new.state.tier, old.state.tier);
    assert_eq!(new.knobs, old.knobs);
    let after = host.creation_checkpoint();
    receipt("forced display move", &before, &after);
    unchanged_creations(&before, &after);
}

#[test]
fn metalfx_resize_builds_only_on_the_extent_worker() {
    let gpu = open("MetalFX resize");
    let old = support::drawable(&gpu.device, FORMAT, EXTENT);
    let output = support::drawable(&gpu.device, FORMAT, RESIZED);
    let mut host = ready(&gpu);
    draw(&mut host, &old, EXTENT);
    let before = host.creation_checkpoint();
    let inputs = host.extent_prepare_inputs(RESIZED).expect("extent inputs");
    let worker = std::thread::Builder::new()
        .name("extent worker".into())
        .spawn(move || {
            inputs
                .build(&AtomicBool::new(false))
                .expect("extent build")
                .expect("extent candidate")
        })
        .expect("worker");
    let candidate = worker.join().expect("worker result");
    assert!(host.publish_extent(candidate));
    let next = draw(&mut host, &output, RESIZED);
    let after = host.creation_checkpoint();
    receipt("MetalFX resize", &before, &after);
    assert_eq!(host.reconstruction(), Reconstruction::MetalFx);
    assert_eq!(next.render_extent, RESIZED);
    assert_eq!(next.history_identity[1], 0);
    assert_eq!(before.pipelines, after.pipelines, "{after}");
    assert_eq!(
        after.native_events(CreationPhase::GenerationBuild),
        1,
        "{after}"
    );
    let native = after.native_creations.last().unwrap();
    assert_eq!(
        (native.thread.as_str(), native.phase),
        ("extent worker", CreationPhase::GenerationBuild)
    );
    assert!(
        after.phase(CreationPhase::GenerationBuild).textures > 0,
        "{after}"
    );
    assert_eq!(
        before.phase(CreationPhase::Play).shader_modules,
        after.phase(CreationPhase::Play).shader_modules
    );
    assert_eq!(after.violations, 0);
}

#[test]
fn lost_device_rebuilds_a_separate_generation_before_play() {
    let gpu = open("device recovery");
    let mut host = ready(&gpu);
    let output = support::drawable(&gpu.device, FORMAT, EXTENT);
    draw(&mut host, &output, EXTENT);
    let before = host.creation_checkpoint();
    let inputs = host
        .device_prepare_inputs()
        .expect("retain device preparation inputs");
    gpu.device.destroy();
    let (device, queue, capabilities) = gpu
        .gpu
        .fresh_adapter()
        .device_with_hardware(Profile::Native, &Restriction::default(), 2, &[])
        .expect("replacement device");
    let capabilities = capabilities
        .with_recipe_override("apple-tile")
        .expect("replacement Apple tile");
    let output = support::drawable(&device, FORMAT, EXTENT);
    let mut candidate =
        pollster::block_on(inputs.prepare(&capabilities, &queue, Some(profile(EXTENT))))
            .expect("recovery preparation");
    assert_eq!(
        host.creation_report().generation,
        1,
        "preparing is not publishing"
    );
    host.publish_device(&mut candidate)
        .expect("publish the prepared device");
    let retired = host.retired_creation_report().unwrap();
    assert_eq!(
        (retired.generation, retired.ready, retired.frames),
        (1, true, 1)
    );
    assert_eq!(retired.pipelines, before.pipelines);
    let ready = host.creation_checkpoint();
    assert_eq!((ready.generation, ready.ready, ready.frames), (2, true, 0));
    assert!(
        ready.pipeline_events(CreationPhase::Recovery) > 0,
        "{ready}"
    );
    assert!(
        ready
            .pipelines
            .iter()
            .all(|row| row.phase == CreationPhase::Recovery
                && row.thread == std::thread::current().name().unwrap()
                && row.label != "unscoped"),
        "{ready}"
    );
    assert_eq!(ready.native_events(CreationPhase::Recovery), 1);
    assert_eq!(draw(&mut host, &output, EXTENT).history_identity[1], 0);
    let after = host.creation_checkpoint();
    receipt("device recovery readiness", &ready, &after);
    unchanged_creations(&ready, &after);
}
