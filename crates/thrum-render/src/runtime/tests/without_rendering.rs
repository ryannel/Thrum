//! Authority keeps its clock and input while a lost GPU remains untouched.

use super::{FrameConfig, FrameHost, FrameWorld, SourceTick, StaticScene, SyntheticScene, device};
use crate::{
    present::SurfaceProfile,
    runtime::{FrameError, SurfaceOutput},
};
use std::{num::NonZeroU32, time::Duration};
use thrum_core::{
    ClockConfig, ClockError, WorldEpoch,
    sim::{
        ClockedWorld, ClockedWorldError, SnapshotId, WorldAdvance, WorldGame, WorldPublication,
        WorldStep,
    },
};

#[derive(Default)]
struct Game {
    tick: u64,
    control: u32,
    position: u64,
    attempts: u32,
    fail_on: Option<u64>,
    recover: bool,
}

impl WorldGame for Game {
    type Control = u32;
    type Page = SourceTick;
    type Error = &'static str;

    fn validate_control(control: u32) -> Result<(), Self::Error> {
        (control <= 3).then_some(()).ok_or("invalid control")
    }
    fn same_control(left: u32, right: u32) -> bool {
        left == right
    }
    fn apply_control(&mut self, control: u32) -> Result<(), Self::Error> {
        self.control = control;
        Ok(())
    }
    fn endpoint(&self) -> (SnapshotId, u64) {
        (
            SnapshotId {
                epoch: WorldEpoch(23),
                tick: self.tick,
            },
            7,
        )
    }
    fn step(&mut self) -> Result<WorldStep, Self::Error> {
        self.attempts += 1;
        if self.fail_on == Some(self.tick + 1) {
            self.fail_on = None;
            return Err("retained failed tick");
        }
        self.tick += 1;
        self.position += u64::from(self.control);
        Ok(WorldStep {
            projected_changes: 1,
            ..WorldStep::default()
        })
    }
    fn recover_publication(&mut self) -> Result<Option<usize>, Self::Error> {
        Ok(std::mem::take(&mut self.recover).then_some(0))
    }
    fn publication(&mut self) -> Result<Option<WorldPublication<Self::Page>>, Self::Error> {
        let (id, timeline) = self.endpoint();
        Ok(Some(WorldPublication {
            id,
            timeline,
            page: SourceTick {
                epoch: id.epoch.0,
                tick: id.tick,
                timeline,
            },
        }))
    }
}

struct World {
    clock: ClockedWorld<Game>,
    publication_calls: u32,
}

impl FrameWorld for World {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        let (id, timeline) = self.clock.game().endpoint();
        SourceTick {
            epoch: id.epoch.0,
            tick: id.tick,
            timeline,
        }
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        self.clock.advance(now).map_err(|error| error.to_string())
    }
    fn publication(&mut self) -> Result<WorldPublication<SourceTick>, String> {
        self.publication_calls += 1;
        let source = self.source();
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(source.epoch),
                tick: source.tick,
            },
            timeline: source.timeline,
            page: source,
        })
    }
    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.clock.presentation_alpha_at(lead)
    }
}

async fn host(cap: u32) -> FrameHost<World, StaticScene> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (capabilities, queue) = device::gpu(&instance, 7).await;
    FrameHost::new(
        &capabilities,
        &queue,
        FrameConfig {
            render_extent: [64, 64],
            surface: Some(device::surface()),
        },
        World {
            clock: ClockedWorld::new(
                Game::default(),
                ClockConfig {
                    tick_hz: NonZeroU32::new(10).expect("tick rate"),
                    max_ticks_per_call: NonZeroU32::new(cap).expect("tick cap"),
                    ..ClockConfig::default()
                },
                Duration::ZERO,
            )
            .expect("clock"),
            publication_calls: 0,
        },
        StaticScene::default(),
        SyntheticScene::grid(1).expect("one diagnostic row"),
        1.0,
    )
    .await
    .expect("prepared host")
}

#[test]
fn destroyed_device_with_pending_work_does_not_block_authority_or_touch_presentation() {
    pollster::block_on(async {
        let mut host = host(1).await;
        let SurfaceProfile { format, extent } = device::surface();
        let target = host.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pre-loss real presentation"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let submitted = host
            .drive(
                Duration::ZERO,
                Some(&SurfaceOutput {
                    view: &view,
                    extent,
                }),
            )
            .expect("actual GPU submission");
        assert!(submitted.submitted && submitted.surface_presented);
        assert_eq!(host.pending_frames(), 1);
        let states: Vec<_> = host
            .slots
            .iter()
            .map(|s| std::mem::discriminant(&s.state))
            .collect();
        let history = (
            host.last_view_source,
            host.history_reset_pending,
            host.upload_reset_pending,
        );
        let before = (
            host.completed.len(),
            host.next_submission_order,
            host.dropped_reports,
            host.source.frames.get(),
            host.world.publication_calls,
            host.scene.instances[0].current,
        );
        let dropped = (host.dropped_views, host.dropped_surface);
        let world_source = host.world.source();
        host.device.destroy();
        host.world.clock.set_controls(3).expect("input after loss");
        let report = host
            .advance_without_rendering(Duration::from_millis(100))
            .expect("authority after loss");
        assert_eq!(
            (report.tick, report.completed_ticks, report.queued_ticks),
            (1, 1, 0)
        );
        assert_eq!(report.projected_changes, 1);
        assert!(report.overload.is_none() && report.dropped_view);
        assert!(!report.submitted && !report.surface_presented);
        assert_eq!(host.world.clock.game().position, 3);
        assert_eq!(
            host.world.source(),
            SourceTick {
                tick: 1,
                ..world_source
            }
        );
        assert_eq!(
            host.pending_frames(),
            1,
            "lost packet is neither polled nor retired"
        );
        assert_eq!(
            host.slots
                .iter()
                .map(|s| std::mem::discriminant(&s.state))
                .collect::<Vec<_>>(),
            states
        );
        assert_eq!(
            (
                host.last_view_source,
                host.history_reset_pending,
                host.upload_reset_pending
            ),
            history
        );
        assert_eq!(
            (
                host.completed.len(),
                host.next_submission_order,
                host.dropped_reports,
                host.source.frames.get(),
                host.world.publication_calls,
                host.scene.instances[0].current
            ),
            before
        );
        assert_eq!(
            (host.dropped_views, host.dropped_surface),
            (dropped.0 + 1, dropped.1 + 1)
        );
        let unchanged = host
            .advance_without_rendering(Duration::from_millis(100))
            .expect("zero ticks");
        assert!(!unchanged.dropped_view);
        host.world.clock.game_mut().recover = true;
        let recovered = host
            .advance_without_rendering(Duration::from_millis(100))
            .expect("recover without tick");
        assert_eq!(
            (recovered.completed_ticks, recovered.projected_changes),
            (0, 0)
        );
        assert!(recovered.dropped_view);
        assert_eq!(
            (host.dropped_views, host.dropped_surface),
            (dropped.0 + 2, dropped.1 + 2)
        );
    });
}

#[test]
fn authority_only_pause_drains_backlog_and_preserves_resume_refusal_and_overload() {
    pollster::block_on(async {
        let mut host = host(1).await;
        let admitted = host
            .advance(Duration::from_millis(300))
            .expect("normal admission");
        assert_eq!(admitted.report.as_ref().expect("report").queued_ticks, 2);
        drop(admitted);
        assert_eq!((host.dropped_views, host.dropped_surface), (1, 1));
        host.world.clock.pause();
        assert!(matches!(
            host.world.clock.resume(Duration::from_millis(300)),
            Err(ClockedWorldError::Clock(ClockError::PendingTicks))
        ));
        for (tick, queued) in [(2, 1), (3, 0)] {
            let report = host
                .advance_without_rendering(Duration::from_secs(300))
                .expect("drain accepted work only");
            assert_eq!(
                (report.tick, report.completed_ticks, report.queued_ticks),
                (tick, 1, queued)
            );
            assert!(report.dropped_view);
        }
        assert_eq!((host.dropped_views, host.dropped_surface), (3, 3));
        assert_eq!(
            host.advance_without_rendering(Duration::from_secs(300))
                .expect("paused gap")
                .completed_ticks,
            0
        );
        host.world
            .clock
            .resume(Duration::from_secs(300))
            .expect("fresh origin");
        let resumed = host
            .advance_without_rendering(Duration::from_millis(300_100))
            .expect("one new tick");
        assert_eq!((resumed.tick, resumed.completed_ticks), (4, 1));
        let overload = host
            .advance_without_rendering(Duration::from_secs(301))
            .expect("refusal is reported");
        assert_eq!(overload.overload, Some(9));
        assert_eq!(overload.completed_ticks, 0);
        assert!(!overload.submitted && !overload.surface_presented);
        let repeated = host
            .advance_without_rendering(Duration::from_secs(301))
            .expect("sticky refusal");
        assert_eq!(repeated.overload, overload.overload);
        assert_eq!(repeated.tick, 4);
    });
}

#[test]
fn invalid_time_and_partial_world_failure_do_not_repeat_committed_ticks() {
    pollster::block_on(async {
        let mut host = host(2).await;
        host.world.clock.set_controls(2).expect("held input");
        host.advance_without_rendering(Duration::from_millis(100))
            .expect("first tick");
        let dropped = host.dropped_views;
        assert!(
            matches!(host.advance_without_rendering(Duration::from_millis(99)), Err(FrameError::World(message)) if message.contains("NonMonotonicTime"))
        );
        assert_eq!(host.world.clock.game().attempts, 1);
        assert_eq!(host.dropped_views, dropped);
        host.world.clock.game_mut().fail_on = Some(3);
        assert!(
            matches!(host.advance_without_rendering(Duration::from_millis(300)), Err(FrameError::World(message)) if message.contains("retained failed tick"))
        );
        assert_eq!(
            (
                host.world.clock.game().tick,
                host.world.clock.game().position
            ),
            (2, 4)
        );
        assert_eq!(host.world.clock.game().attempts, 3);
        let retry = host
            .advance_without_rendering(Duration::from_millis(300))
            .expect("retained third tick");
        assert_eq!(
            (retry.tick, retry.completed_ticks, retry.queued_ticks),
            (3, 1, 0)
        );
        assert_eq!(
            (
                host.world.clock.game().position,
                host.world.clock.game().attempts
            ),
            (6, 4)
        );
        assert_eq!(host.pending_frames(), 0);
    });
}
