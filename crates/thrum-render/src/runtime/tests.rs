use super::{FrameSample, IN_FLIGHT, pop_report_before_pending, retain_sample};
use std::{collections::VecDeque, time::Duration};

use super::{FrameConfig, FrameHost, FrameWorld, SceneFrame, SceneSource};
use crate::{
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene, SyntheticView},
};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};

fn sample(tick: u64) -> FrameSample {
    FrameSample {
        tick,
        presentation_alpha: 1.0,
        submission_order: tick,
        completed_ticks: 1,
        queued_ticks: 0,
        tick_cpu: Duration::ZERO,
        extract_cpu: Duration::ZERO,
        visual_age_ticks: 0,
        upload_cpu: Duration::ZERO,
        view_update_ns: 0,
        encode_cpu: Duration::ZERO,
        wall: Duration::ZERO,
        render_cpu: Duration::ZERO,
        gpu_pass_ns: None,
        gpu_present_ns: None,
        surface_presented: false,
        timing_fault: false,
        uploaded_rows: 0,
        uploaded_bytes: 0,
        moving: 0,
        dropped_presentation: 0,
        dropped_light_references: 0,
        clamped_light_channels: 0,
        clamped_shadow_casters: 0,
        visual_revision: None,
    }
}

#[test]
fn reused_source_ticks_do_not_hide_earlier_completed_submissions() {
    let mut queue = VecDeque::with_capacity(IN_FLIGHT);
    let mut dropped = 0;
    let mut first = sample(1);
    first.submission_order = 7;
    let mut second = sample(1);
    second.submission_order = 8;
    retain_sample(&mut queue, &mut dropped, first);
    assert_eq!(
        pop_report_before_pending(&mut queue, Some(second.submission_order))
            .expect("earlier completed submission")
            .submission_order,
        first.submission_order
    );
    assert!(queue.is_empty());
    retain_sample(&mut queue, &mut dropped, second);
    assert!(pop_report_before_pending(&mut queue, Some(7)).is_none());
    assert_eq!(
        pop_report_before_pending(&mut queue, None)
            .expect("remaining submission")
            .submission_order,
        second.submission_order
    );
    assert_eq!(dropped, 0);
}

#[test]
fn full_report_queue_drops_the_oldest_tick_even_when_it_finishes_late() {
    assert_eq!(IN_FLIGHT, 2);
    let mut queue = VecDeque::with_capacity(IN_FLIGHT);
    let mut dropped = 0;
    for tick in [2, 3] {
        retain_sample(&mut queue, &mut dropped, sample(tick));
    }
    retain_sample(&mut queue, &mut dropped, sample(1));
    assert_eq!(dropped, 1);
    assert_eq!(
        queue.iter().map(|entry| entry.tick).collect::<Vec<_>>(),
        [2, 3]
    );
    retain_sample(&mut queue, &mut dropped, sample(4));
    assert_eq!(dropped, 2);
    assert_eq!(
        queue.iter().map(|entry| entry.tick).collect::<Vec<_>>(),
        [3, 4]
    );
}

struct StaticWorld {
    authority: SourceTick,
    publication: SourceTick,
}

impl FrameWorld for StaticWorld {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        self.authority
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _now: Duration) -> Result<WorldAdvance, String> {
        Ok(WorldAdvance {
            tick: self.authority.tick,
            overload: None,
            completed_ticks: 0,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, String> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(self.publication.epoch),
                tick: self.publication.tick,
            },
            timeline: self.publication.timeline,
            page: self.publication,
        })
    }
    fn presentation_alpha_at(&self, _lead: Duration) -> f32 {
        1.0
    }
}

#[derive(Default)]
struct StaticScene {
    frames: std::cell::Cell<u32>,
    fail_next: std::cell::Cell<bool>,
    invalid_lighting: std::cell::Cell<bool>,
    offset: f64,
}

impl SceneSource<SourceTick> for StaticScene {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            let tick = u32::try_from(page.tick).expect("bounded fixture tick");
            sink.push(0, 1, 7, [f64::from(tick) * 0.1 + self.offset, 0.0, 0.5], 0);
        })
    }
    fn frame(
        &self,
        _source: SourceTick,
        _alpha: f32,
        _extent: [u32; 2],
        _lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        self.frames.set(self.frames.get() + 1);
        if self.fail_next.replace(false) {
            return Err("presentation policy refused this sample".to_owned());
        }
        let mut light = SyntheticLight::default();
        if self.invalid_lighting.get() {
            light.irradiance = [f32::NAN; 3];
        }
        Ok(SceneFrame {
            view: SyntheticView::default(),
            lighting: FrameLighting::Diagnostic(light),
        })
    }
}

#[test]
fn lagging_or_replayed_publication_is_shed_before_camera_scene_mutation_or_submission() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native GPU adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 7);
        let published = SourceTick {
            epoch: 2,
            tick: 0,
            timeline: 1,
        };
        let world = StaticWorld {
            authority: published,
            publication: published,
        };
        let scene = SyntheticScene::grid(1).expect("one diagnostic row");
        let mut host = FrameHost::new(
            &capabilities,
            &queue,
            FrameConfig::default(),
            world,
            StaticScene::default(),
            scene,
            1.0,
        )
        .await
        .expect("prepared engine host");
        let before = host.scene.instances[0].current;
        for (index, authority) in [
            SourceTick {
                tick: 1,
                ..published
            },
            SourceTick {
                timeline: 2,
                ..published
            },
        ]
        .into_iter()
        .enumerate()
        {
            host.world.authority = authority;
            let frame = host
                .extract_serial(Duration::ZERO, 0, 0)
                .expect("real publication handoff");
            assert_eq!(frame.source, published, "old page keeps its exact source");
            let mut report = super::DriveReport {
                tick: authority.tick,
                overload: None,
                completed_ticks: 0,
                queued_ticks: 0,
                projected_changes: 0,
                projection_lost_ticks: 0,
                submitted: false,
                surface_presented: false,
                dropped_view: false,
                wall: Duration::ZERO,
            };
            host.submit_extracted(0, &mut report, None, frame, super::Instant::now())
                .expect("stale presentation degrades");
            assert!(report.dropped_view);
            assert!(!report.submitted);
            assert_eq!(
                host.dropped_stale_views(),
                u64::try_from(index + 1).expect("bounded cases")
            );
            assert_eq!(
                host.source.frames.get(),
                1,
                "stale pages never sample a newer camera"
            );
            assert_eq!(host.scene.instances[0].current, before);
            assert_eq!(host.pending_frames(), 0);
        }
    });
}

async fn prepared_host() -> FrameHost<StaticWorld, StaticScene> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("native GPU adapter");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("device");
    let capabilities = DeviceCapabilities::capture(&adapter, &device, 7);
    let source = SourceTick {
        epoch: 2,
        tick: 0,
        timeline: 1,
    };
    FrameHost::new(
        &capabilities,
        &queue,
        FrameConfig::default(),
        StaticWorld {
            authority: source,
            publication: source,
        },
        StaticScene::default(),
        SyntheticScene::grid(1).expect("one diagnostic row"),
        1.0,
    )
    .await
    .expect("prepared engine host")
}

fn progress(host: &FrameHost<StaticWorld, StaticScene>) -> super::DriveReport {
    super::DriveReport {
        tick: host.world.source().tick,
        overload: None,
        completed_ticks: 0,
        queued_ticks: 0,
        projected_changes: 0,
        projection_lost_ticks: 0,
        submitted: false,
        surface_presented: false,
        dropped_view: false,
        wall: Duration::ZERO,
    }
}

#[test]
fn refused_policy_keeps_changed_rows_pending_for_a_same_source_retry() {
    pollster::block_on(async {
        let mut host = prepared_host().await;
        let before = host.scene.instances[0].current;
        host.world.authority.tick = 1;
        host.world.publication = host.world.authority;
        host.source.fail_next.set(true);
        let first = host
            .extract_serial(Duration::ZERO, 0, 0)
            .expect("changed page");
        let mut report = progress(&host);
        assert!(matches!(
            host.prepare_extracted(0, &mut report, first),
            Err(super::FrameError::Scene(_))
        ));
        assert_eq!(
            host.scene.instances[0].current, before,
            "policy refusal precedes scene mutation"
        );
        let retry = host
            .extract_serial(Duration::ZERO, 0, 0)
            .expect("same page retry");
        let prepared = host
            .prepare_extracted(0, &mut report, retry)
            .expect("valid sample")
            .expect("current source");
        assert_eq!(
            prepared.sample.uploaded_rows, 1,
            "the changed transform still reaches upload"
        );
        assert_ne!(host.scene.instances[0].current, before);
    });
}

#[test]
fn invalid_lighting_retains_upload_work_after_cpu_scene_mutation() {
    pollster::block_on(async {
        let mut host = prepared_host().await;
        host.world.authority.tick = 1;
        host.world.publication = host.world.authority;
        host.source.invalid_lighting.set(true);
        let first = host
            .extract_serial(Duration::ZERO, 0, 0)
            .expect("changed page");
        let mut report = progress(&host);
        assert!(matches!(
            host.prepare_extracted(0, &mut report, first),
            Err(super::FrameError::Gpu(_))
        ));
        host.source.invalid_lighting.set(false);
        let retry = host
            .extract_serial(Duration::ZERO, 0, 0)
            .expect("same page retry");
        let prepared = host
            .prepare_extracted(0, &mut report, retry)
            .expect("repaired lighting")
            .expect("current source");
        assert_eq!(
            prepared.sample.uploaded_rows, 1,
            "an already updated CPU scene must still repair the GPU delta"
        );
        assert!(host.history_reset_pending);
    });
}

#[test]
fn changing_mapping_at_a_fixed_world_source_reextracts_and_uploads_the_new_rows() {
    pollster::block_on(async {
        let mut host = prepared_host().await;
        let before = host.scene.instances[0].current;
        host.source_mut().offset = 0.5;
        let frame = host
            .extract_serial(Duration::ZERO, 0, 0)
            .expect("same endpoint with changed mapping");
        assert!(!frame.reused_source);
        let mut report = progress(&host);
        let prepared = host
            .prepare_extracted(0, &mut report, frame)
            .expect("changed policy")
            .expect("current source");
        assert_eq!(prepared.sample.tick, 0);
        assert_eq!(prepared.sample.uploaded_rows, 1);
        assert_ne!(host.scene.instances[0].current, before);
        assert!(host.history_reset_pending);
    });
}
