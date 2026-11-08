//! G3t D0.3 witnesses: pipeline events on the generation-owned site, by thread
//! and declared phase, the play guard, and the live-object delta diagnostic.
//!
//! Each ledger is reached only through its own [`DeviceSite`], so every
//! assertion reads exact totals: no thread filtering, and tests running in
//! parallel on their own instances cannot reach one another's ledgers.

use super::{Duration, FrameLighting, Host, Policy, SyntheticLight, gpu, host, surface};
use crate::capabilities::DeviceCapabilities;
use crate::runtime::{
    CreationPhase, DeviceSite, Instrumentation, NativeSites, PipelineKind, PipelineRow,
};
use std::time::Instant;

fn retire(host: &mut Host) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.pending_frames() != 0 {
        let timeout = deadline.saturating_duration_since(Instant::now());
        assert!(!timeout.is_zero(), "creation test frame did not retire");
        host.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(timeout),
            })
            .expect("wait for frame");
        while host.poll_completed().expect("collect frame").is_some() {}
    }
}

/// The host generation's site, as its capabilities hold it.
fn site(host: &Host) -> DeviceSite {
    host.prepared.capabilities().site().clone()
}

/// The test's own drawable, charged to a fixture transition rather than play.
fn drawable(host: &Host, extent: [u32; 2]) -> wgpu::Texture {
    let profile = surface();
    let _fixture = host.creations.region(CreationPhase::Transition, "fixture");
    host.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("creation-test drawable"),
        size: wgpu::Extent3d {
            width: extent[0],
            height: extent[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: profile.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

/// Submit and retire `frames` presented frames.
fn play(host: &mut Host, texture: &wgpu::Texture, frames: usize) {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let output = super::super::super::SurfaceOutput {
        view: &view,
        extent: [texture.width(), texture.height()],
    };
    for _ in 0..frames {
        let report = host.drive(Duration::ZERO, Some(&output)).expect("frame");
        assert!(report.submitted && report.surface_presented);
        retire(host);
    }
}

/// A pipeline built through the production seam: the lazy hitch the guard
/// exists for when no region is declared.
fn lazy_pipeline(site: &DeviceSite) -> wgpu::ComputePipeline {
    let module = site.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("intentional shader"),
        source: wgpu::ShaderSource::Wgsl("@compute @workgroup_size(1) fn main() {}".into()),
    });
    site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("intentional pipeline"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn this_thread() -> String {
    std::thread::current()
        .name()
        .expect("libtest names its threads")
        .to_owned()
}

fn pipeline_row<'a>(rows: &'a [PipelineRow], label: &str) -> Option<&'a PipelineRow> {
    rows.iter().find(|row| row.label == label)
}

/// Close a live-object window, so the deltas so far are charged.
fn flush(host: &Host) {
    drop(host.creations.region(CreationPhase::Transition, "flush"));
}

#[test]
fn play_builds_nothing_and_transitions_are_charged_to_their_thread() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        let report = host.creation_report();
        assert_eq!(report.generation, 7);
        assert!(report.ready);
        assert_eq!(report.native, NativeSites::Absent);
        assert_eq!(report.instrumentation, Instrumentation::Counted, "{report}");
        let build = pipeline_row(&report.pipelines, "host").expect("host build events");
        assert_eq!(
            (build.phase, build.thread.as_str()),
            (CreationPhase::Build, this_thread().as_str())
        );
        // The events count every pipeline the build created through the seam,
        // and agree with the live delta of the same serialized window.
        let delta = report.phase(CreationPhase::Build);
        assert_eq!(
            i64::try_from(build.render),
            Ok(delta.render_pipelines),
            "{report}"
        );
        assert_eq!(
            i64::try_from(build.compute),
            Ok(delta.compute_pipelines),
            "{report}"
        );

        let texture = drawable(&host, [64, 64]);
        play(&mut host, &texture, 6);
        host.resize_surface([96, 48]).expect("resize");
        let resized = drawable(&host, [96, 48]);
        play(&mut host, &resized, 2);
        let _capture = host.capture_pixels().expect("capture");
        play(&mut host, &resized, 2);

        // A visual generation prepared on another thread shares the
        // generation's site, and is a transition there.
        let inputs = host.visual_prepare_inputs();
        let prepared = std::thread::Builder::new()
            .name("visual-preparation".to_owned())
            .spawn(move || {
                pollster::block_on(inputs.prepare(
                    FrameLighting::Diagnostic(SyntheticLight::default()),
                    1.0,
                    None,
                ))
            })
            .expect("spawn")
            .join()
            .expect("join");
        let mut prepared = prepared.expect("visual candidate");
        assert!(host.publish_visual(&mut prepared).expect("publish"));
        drop(prepared);
        play(&mut host, &resized, 4);
        flush(&host);

        let report = host.creation_report();
        assert_eq!(report.frames, 14, "{report}");
        assert_eq!(report.violations, 0, "{report}");
        assert_eq!(report.pipeline_events(CreationPhase::Play), 0, "{report}");
        let prepare = pipeline_row(&report.pipelines, "prepare-visual").expect("prepare events");
        assert_eq!(prepare.phase, CreationPhase::Transition);
        assert_eq!(prepare.thread, "visual-preparation");
        assert!(prepare.render > 0, "{report}");
        // Live deltas: the detached preparation's windows are unattributed;
        // every serialized host window is attributed.
        let delta = report
            .rows
            .iter()
            .find(|row| row.label == "prepare-visual")
            .expect("prepare delta");
        assert_eq!(delta.unattributed, delta.windows, "{report}");
        assert!(
            report
                .rows
                .iter()
                .filter(|row| row.label != "prepare-visual")
                .all(|row| row.unattributed == 0 && row.thread == this_thread()),
            "{report}"
        );
        println!("creation report: {report}");
    });
}

#[test]
fn retained_and_created_then_dropped_play_pipelines_both_fail_the_guard() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        let site = site(&host);
        let texture = drawable(&host, [64, 64]);
        play(&mut host, &texture, 3);
        assert_eq!(host.creation_report().violations, 0);

        // Retained: built and kept between frames.
        let kept = lazy_pipeline(&site);
        play(&mut host, &texture, 1);
        let report = host.creation_report();
        assert_eq!(report.violations, 1, "{report}");
        let violation = &report.first_violations[0];
        assert_eq!(
            (
                violation.thread.as_str(),
                violation.after_frame,
                violation.kind
            ),
            (this_thread().as_str(), 3, PipelineKind::Compute)
        );

        // Created then dropped inside one window: the live delta nets to zero,
        // the event does not.
        flush(&host);
        let delta_before = host.creation_report().phase(CreationPhase::Play);
        drop(lazy_pipeline(&site));
        play(&mut host, &texture, 1);
        flush(&host);
        let report = host.creation_report();
        assert_eq!(report.violations, 2, "{report}");
        assert_eq!(
            report.phase(CreationPhase::Play).compute_pipelines,
            delta_before.compute_pipelines,
            "the live delta misses a create-and-drop: {report}"
        );

        // The same creation inside a declared transition is not a violation.
        let region = host
            .creations
            .region(CreationPhase::Transition, "intentional");
        let also_kept = lazy_pipeline(&site);
        drop(region);
        play(&mut host, &texture, 1);
        let report = host.creation_report();
        assert_eq!(report.violations, 2, "{report}");
        let row = pipeline_row(&report.pipelines, "intentional").expect("scoped event");
        assert_eq!((row.phase, row.compute), (CreationPhase::Transition, 1));
        drop((kept, also_kept));
    });
}

#[test]
fn a_cloned_record_shares_the_site_and_a_recaptured_handle_reaches_nothing() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        let texture = drawable(&host, [64, 64]);
        play(&mut host, &texture, 1);
        // Re-capturing the generation's raw handle is no identity: its site is
        // untracked, so nothing reaches the generation's ledger.
        let recaptured = DeviceCapabilities::capture_device(&host.device, 7);
        assert!(recaptured.matches(&host.device));
        let raw = DeviceSite::untracked(host.device.clone());
        let unseen = (lazy_pipeline(recaptured.site()), lazy_pipeline(&raw));
        play(&mut host, &texture, 1);
        let report = host.creation_report();
        assert_eq!(report.violations, 0, "{report}");
        assert_eq!(report.pipeline_events(CreationPhase::Play), 0, "{report}");
        // A clone of the captured record is the same generation.
        let cloned = capabilities.clone();
        let seen = lazy_pipeline(cloned.site());
        play(&mut host, &texture, 1);
        let report = host.creation_report();
        assert_eq!(report.violations, 1, "{report}");
        drop((unseen, seen));
    });
}

#[test]
fn a_same_id_device_of_another_instance_never_reaches_the_ledger() {
    pollster::block_on(async {
        let first = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let second = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&first, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        let texture = drawable(&host, [64, 64]);
        play(&mut host, &texture, 1);
        let before = host.creation_report();
        // The other instance's first device aliases the host's under wgpu's
        // per-instance device identity.
        let (other, _other_queue) = gpu(&second, 7).await;
        assert!(
            **other.site() == host.device,
            "the control needs devices wgpu calls equal"
        );
        // Unregistered: a raw handle of it, unscoped and inside a host region.
        let unregistered = DeviceSite::untracked(wgpu::Device::clone(other.site()));
        let unscoped = lazy_pipeline(&unregistered);
        let region = host
            .creations
            .region(CreationPhase::Transition, "host-only");
        let scoped = lazy_pipeline(&unregistered);
        // Its captured site has a ledger of its own, which no host reads.
        let captured = lazy_pipeline(other.site());
        drop(region);
        play(&mut host, &texture, 1);
        let after = host.creation_report();
        assert_eq!(after.violations, 0, "{after}");
        assert_eq!(after.pipelines, before.pipelines, "{after}");
        drop((unscoped, scoped, captured));
    });
}

#[test]
fn two_registered_same_id_devices_on_one_thread_keep_their_own_events() {
    pollster::block_on(async {
        let first = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let second = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (a_capabilities, a_queue) = gpu(&first, 7).await;
        let (b_capabilities, b_queue) = gpu(&second, 7).await;
        let mut a = host(&a_capabilities, &a_queue, Policy::default()).await;
        let mut b = host(&b_capabilities, &b_queue, Policy::default()).await;
        assert!(
            a.device == b.device,
            "the control needs devices wgpu calls equal"
        );
        let (a_site, b_site) = (site(&a), site(&b));
        let a_texture = drawable(&a, [64, 64]);
        let b_texture = drawable(&b, [64, 64]);
        play(&mut a, &a_texture, 1);
        play(&mut b, &b_texture, 1);
        let a_built = a.creation_report().pipelines;
        // Interleaved on this thread: B inside A's region is B's unscoped
        // creation; A's own unscoped creation is A's; B's scoped one is B's.
        let a_region = a.creations.region(CreationPhase::Transition, "a-region");
        let b_in_a = lazy_pipeline(&b_site);
        drop(a_region);
        let a_unscoped = lazy_pipeline(&a_site);
        let b_region = b.creations.region(CreationPhase::Transition, "b-region");
        let b_scoped = lazy_pipeline(&b_site);
        drop(b_region);
        play(&mut a, &a_texture, 1);
        play(&mut b, &b_texture, 1);
        let a_report = a.creation_report();
        let b_report = b.creation_report();
        assert_eq!(a_report.violations, 1, "{a_report}");
        assert_eq!(b_report.violations, 1, "{b_report}");
        let a_new: Vec<_> = a_report
            .pipelines
            .iter()
            .filter(|row| !a_built.contains(row))
            .collect();
        assert_eq!(a_new.len(), 1, "{a_report}");
        assert_eq!(
            (a_new[0].phase, a_new[0].label, a_new[0].compute),
            (CreationPhase::Play, "unscoped", 1)
        );
        assert!(
            pipeline_row(&a_report.pipelines, "a-region").is_none(),
            "{a_report}"
        );
        let b_unscoped = pipeline_row(&b_report.pipelines, "unscoped").expect("b unscoped");
        assert_eq!(
            (b_unscoped.phase, b_unscoped.compute),
            (CreationPhase::Play, 1)
        );
        let b_row = pipeline_row(&b_report.pipelines, "b-region").expect("b scoped");
        assert_eq!((b_row.phase, b_row.compute), (CreationPhase::Transition, 1));
        drop((b_in_a, a_unscoped, b_scoped));
    });
}

#[test]
fn a_detached_preparation_overlapping_play_separates_its_build_from_an_unscoped_create() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        let texture = drawable(&host, [64, 64]);
        let detached = host.creations.detach();
        let worker_site = site(&host);
        let (opened, wait_opened) = std::sync::mpsc::channel();
        let (release, wait_release) = std::sync::mpsc::channel::<()>();
        let preparer = std::thread::Builder::new()
            .name("preparer".to_owned())
            .spawn(move || {
                let region = detached.region(CreationPhase::Transition, "overlap");
                opened.send(()).expect("opened");
                wait_release.recv().expect("release");
                // A legitimate scoped build on the preparer, after the host's
                // creation and frame below, inside the same region.
                let built = lazy_pipeline(&worker_site);
                drop(region);
                built
            })
            .expect("spawn");
        wait_opened.recv().expect("region open");
        // An illicit unscoped creation on the host thread while the preparer's
        // region is open, then a frame, all before the preparer builds.
        let illicit = lazy_pipeline(&site(&host));
        play(&mut host, &texture, 1);
        release.send(()).expect("release");
        let built = preparer.join().expect("join");
        play(&mut host, &texture, 1);
        flush(&host);
        let report = host.creation_report();
        assert_eq!(report.violations, 1, "{report}");
        let violation = &report.first_violations[0];
        assert_eq!(
            (violation.thread.as_str(), violation.after_frame),
            (this_thread().as_str(), 0)
        );
        let scoped = pipeline_row(&report.pipelines, "overlap").expect("scoped build");
        assert_eq!(
            (scoped.phase, scoped.thread.as_str(), scoped.compute),
            (CreationPhase::Transition, "preparer", 1),
            "{report}"
        );
        let unscoped = pipeline_row(&report.pipelines, "unscoped").expect("unscoped event");
        assert_eq!(
            (unscoped.phase, unscoped.thread.as_str(), unscoped.compute),
            (CreationPhase::Play, this_thread().as_str(), 1),
            "{report}"
        );
        // The live-delta diagnostic cannot separate them: both are unattributed.
        assert!(
            report
                .rows
                .iter()
                .filter(|row| row.phase != CreationPhase::Build)
                .filter(|row| row.counts.compute_pipelines != 0)
                .all(|row| row.unattributed == row.windows),
            "{report}"
        );
        drop((illicit, built));
    });
}

#[test]
fn a_replacement_device_starts_its_own_ledger_and_retires_the_old_one() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, queue) = gpu(&instance, 7).await;
        let mut host = host(&first, &queue, Policy::default()).await;
        let texture = drawable(&host, [64, 64]);
        play(&mut host, &texture, 2);
        let old_events = host.creation_report().pipelines;
        let inputs = host.device_prepare_inputs().expect("inputs");
        let (second, second_queue) = gpu(&instance, 8).await;
        let mut candidate = inputs
            .prepare(&second, &second_queue, Some(surface()))
            .await
            .expect("candidate");
        candidate.resize_surface([96, 48]).expect("refit");
        host.publish_device(&mut candidate).expect("publish");
        let retired = host.retired_creation_report().expect("retired").clone();
        assert_eq!((retired.generation, retired.frames), (7, 2));
        assert_eq!(retired.violations, 0);
        assert_eq!(
            retired.pipelines, old_events,
            "the replacement's build never reached generation 7"
        );
        let report = host.creation_report();
        assert_eq!(report.generation, 8);
        assert!(report.ready);
        assert_eq!(report.instrumentation, Instrumentation::Counted, "{report}");
        assert!(
            report
                .pipelines
                .iter()
                .all(|row| row.phase == CreationPhase::Build && row.label != "unscoped"),
            "{report}"
        );
        assert!(
            pipeline_row(&report.pipelines, "device").is_some(),
            "{report}"
        );
        let texture = drawable(&host, [96, 48]);
        play(&mut host, &texture, 3);
        let report = host.creation_report();
        assert_eq!((report.frames, report.violations), (3, 0), "{report}");
    });
}

/// The frame path's cost with the diagnostic on: one relaxed atomic add.
/// Run with `--ignored --nocapture` in release.
#[test]
#[ignore = "measurement; run in release"]
fn measure_the_per_frame_ledger_cost() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let host = host(&capabilities, &queue, Policy::default()).await;
        let mut runs = Vec::new();
        for _ in 0..9 {
            let start = Instant::now();
            for _ in 0..100_000 {
                host.creations.frame();
            }
            runs.push(start.elapsed().as_nanos() as f64 / 100_000.0);
        }
        runs.sort_by(f64::total_cmp);
        println!(
            "ledger frame(): min {:.1} ns, p50 {:.1} ns, max {:.1} ns per call",
            runs[0], runs[4], runs[8]
        );
    });
}

type Compiles =
    std::sync::Arc<std::sync::Mutex<Vec<std::thread::JoinHandle<wgpu::ComputePipeline>>>>;

/// A contribution shaped like the cooked one: it builds a pipeline through
/// `SceneSurface::device` in every rebuild, and after a device or surface
/// replacement restores on a compile thread under `"material-restore"`.
struct Compiling {
    rebuilds: usize,
    built: Vec<wgpu::ComputePipeline>,
    compiles: Compiles,
}

/// One compile on a `thrum-cooked-material` thread, declared as the cooked
/// material compile queue declares it, or not at all.
fn compile_thread(
    site: DeviceSite,
    label: Option<&'static str>,
) -> std::thread::JoinHandle<wgpu::ComputePipeline> {
    std::thread::Builder::new()
        .name("thrum-cooked-material".to_owned())
        .spawn(move || {
            let region = label.map(|label| site.transition(label));
            let pipeline = lazy_pipeline(&site);
            drop(region);
            pipeline
        })
        .expect("spawn compile")
}

impl crate::contribution::SceneContribution for Compiling {
    fn passes(&self) -> crate::contribution::ContributionPasses {
        crate::contribution::ContributionPasses::default()
    }
    fn rebuild(&mut self, surface: &crate::contribution::SceneSurface<'_>) -> Result<(), String> {
        self.rebuilds += 1;
        self.built.push(lazy_pipeline(surface.device));
        if self.rebuilds > 1 {
            let restore = compile_thread(surface.device.clone(), Some("material-restore"));
            self.compiles.lock().expect("compiles").push(restore);
        }
        Ok(())
    }
    fn encode_upload(
        &mut self,
        _: &mut wgpu::CommandEncoder,
        _: &crate::contribution::ContributionFrame,
    ) {
    }
    fn computes(&self, _: &crate::contribution::ContributionFrame) -> bool {
        false
    }
    fn encode_compute(
        &mut self,
        _: &mut wgpu::ComputePass<'_>,
        _: &crate::contribution::ContributionFrame,
    ) {
    }
    fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {}
    fn draw(&self, _: &mut wgpu::RenderPass<'_>) {}
    fn submitted(&mut self, _: bool) {}
}

fn join(compiles: &Compiles) -> Vec<wgpu::ComputePipeline> {
    let handles = std::mem::take(&mut *compiles.lock().expect("compiles"));
    handles
        .into_iter()
        .map(|handle| handle.join().expect("compile"))
        .collect()
}

#[test]
fn contribution_rebuilds_and_compile_thread_reloads_and_restores_are_declared() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, queue) = gpu(&instance, 7).await;
        let mut host = host(&first, &queue, Policy::default()).await;
        let compiles = Compiles::default();
        host.set_scene_contribution(Box::new(Compiling {
            rebuilds: 0,
            built: Vec::new(),
            compiles: std::sync::Arc::clone(&compiles),
        }))
        .expect("attach");
        let texture = drawable(&host, [64, 64]);
        play(&mut host, &texture, 2);
        // A reload during play, on the compile thread, declared there.
        let reloaded = compile_thread(site(&host), Some("material-reload"))
            .join()
            .expect("reload");
        play(&mut host, &texture, 1);
        let report = host.creation_report();
        assert_eq!(report.violations, 0, "{report}");
        let attach = pipeline_row(&report.pipelines, "contribution").expect("attach");
        assert_eq!(
            (attach.phase, attach.compute),
            (CreationPhase::Transition, 1)
        );
        let reload = pipeline_row(&report.pipelines, "material-reload").expect("reload");
        assert_eq!(
            (reload.phase, reload.thread.as_str(), reload.compute),
            (CreationPhase::Transition, "thrum-cooked-material", 1)
        );
        // The same compile undeclared is a play violation on that thread.
        let undeclared = compile_thread(site(&host), None).join().expect("compile");
        play(&mut host, &texture, 1);
        let report = host.creation_report();
        assert_eq!(report.violations, 1, "{report}");
        assert_eq!(report.first_violations[0].thread, "thrum-cooked-material");

        // A failed replacement attempt's ledger never receives the host's events.
        let (failed, _failed_queue) = gpu(&instance, 9).await;
        let inputs = host.device_prepare_inputs().expect("inputs");
        let (second, second_queue) = gpu(&instance, 8).await;
        let mut candidate = inputs
            .prepare(&second, &second_queue, Some(surface()))
            .await
            .expect("candidate");
        candidate.resize_surface([96, 48]).expect("refit");
        host.publish_device(&mut candidate).expect("publish");
        let restored = join(&compiles);
        assert_eq!(restored.len(), 1, "one restore per replacement");
        let texture = drawable(&host, [96, 48]);
        play(&mut host, &texture, 2);
        let report = host.creation_report();
        assert_eq!((report.generation, report.violations), (8, 0), "{report}");
        let rebuild = pipeline_row(&report.pipelines, "publish-device").expect("rebuild");
        assert_eq!((rebuild.phase, rebuild.compute), (CreationPhase::Build, 1));
        let restore = pipeline_row(&report.pipelines, "material-restore").expect("restore");
        assert_eq!(
            (restore.phase, restore.thread.as_str(), restore.compute),
            (CreationPhase::Transition, "thrum-cooked-material", 1)
        );
        // A contribution held by the app and attached right after publication is
        // a declared transition on the new generation.
        host.set_scene_contribution(Box::new(Compiling {
            rebuilds: 0,
            built: Vec::new(),
            compiles: std::sync::Arc::clone(&compiles),
        }))
        .expect("attach after publish");
        play(&mut host, &texture, 1);
        let report = host.creation_report();
        assert_eq!((report.generation, report.violations), (8, 0), "{report}");
        let attach = pipeline_row(&report.pipelines, "contribution").expect("attach");
        assert_eq!(
            (attach.phase, attach.compute),
            (CreationPhase::Transition, 1)
        );
        let retired = host.retired_creation_report().expect("retired");
        assert!(
            pipeline_row(&retired.pipelines, "material-restore").is_none(),
            "{retired}"
        );
        assert_eq!(retired.violations, 1, "{retired}");
        let unused = failed.site().ledger().report();
        assert!(
            unused.pipelines.is_empty() && unused.violations == 0,
            "{unused}"
        );
        drop((reloaded, undeclared, restored));
    });
}
