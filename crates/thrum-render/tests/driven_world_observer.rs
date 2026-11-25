//! Device proof of driven-world frame sampling.
use std::{
    cell::Cell,
    convert::Infallible,
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::Duration,
};

use thrum_core::{
    ClockSample, WorldEpoch,
    sim::{DrivenAdvance, DrivenWorld, SnapshotId, WorldDriver, WorldPublication},
};
use thrum_render::{
    capabilities::{DeviceRecipe, OpenedDevice},
    contribution::{
        ContributionFrame, ContributionPasses, EffectTime, SceneContribution, SceneSurface,
        SpriteContributionSurface,
    },
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        FrameConfig, FrameError, FrameHost, SceneFrame, SceneSource, SpriteSceneRequests,
        SurfaceOutput,
    },
    sprite::{EmbeddedSheet, SpriteFrame, SpriteRect, SpriteScene, SpriteView},
    synthetic::FrameLighting,
};

const TICK: Duration = Duration::from_millis(40);

#[derive(Default)]
struct Script(u64);

impl WorldDriver for Script {
    type Control = ();
    type Page = ();
    type Error = Infallible;

    fn validate_control((): ()) -> Result<(), Self::Error> {
        Ok(())
    }
    fn same_control((): (), (): ()) -> bool {
        true
    }
    fn apply_control(&mut self, (): ()) -> Result<(), Self::Error> {
        Ok(())
    }

    fn endpoint(&self) -> (SnapshotId, u64) {
        (
            SnapshotId {
                epoch: WorldEpoch(3),
                tick: self.0,
            },
            0,
        )
    }

    fn advance(&mut self, now: Duration, _: NonZeroU32) -> Result<DrivenAdvance, Self::Error> {
        let (completed_ticks, fraction, rate) = match now.as_millis() {
            5 => (1, 0.125, 25.0),
            15 => (0, 0.375, 25.0),
            25 => (0, 0.625, 25.0),
            35 => (0, 1.0, 50.0),
            45 => (2, 0.5, 27.5),
            80 => (1, 0.25, 25.0),
            _ => unreachable!("script timestamp"),
        };
        self.0 += u64::from(completed_ticks);
        Ok(DrivenAdvance {
            completed_ticks,
            presentation: ClockSample::driven(now, fraction, rate, TICK).expect("phase"),
            next_due: Duration::from_millis(10),
        })
    }

    fn advance_for_display(
        &mut self,
        now: Duration,
        display_at: Duration,
        cap: NonZeroU32,
    ) -> Result<DrivenAdvance, Self::Error> {
        let mut advance = self.advance(now, cap)?;
        if now == Duration::from_millis(45) {
            assert_eq!(display_at, Duration::from_millis(50));
            advance.presentation =
                ClockSample::driven(display_at, 0.55, 27.5, TICK).expect("phase");
        }
        Ok(advance)
    }

    fn publication(&mut self) -> Result<Option<WorldPublication<()>>, Self::Error> {
        let (id, timeline) = self.endpoint();
        Ok(Some(WorldPublication {
            id,
            timeline,
            page: (),
        }))
    }
}

#[derive(Default)]
struct Scene(Cell<Option<(u64, f32)>>);

impl SceneSource<()> for Scene {
    fn extract(&self, _: &mut RenderExtractor, (): &()) -> ExtractReport {
        unreachable!("2D scene")
    }

    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        unreachable!("2D scene")
    }

    fn sprites(
        &self,
        source: SourceTick,
        alpha: f32,
        _: [u32; 2],
    ) -> Result<SpriteFrame<'_>, FrameError> {
        self.0.set(Some((source.tick, alpha)));
        Ok(SpriteFrame {
            view: SpriteView {
                centre: [0.0; 2],
                pixels_per_unit: 1.0,
            },
            rows: &[],
        })
    }
}

struct EffectObserver(Arc<Mutex<Option<EffectTime>>>);

impl SceneContribution for EffectObserver {
    fn draw_stages(&self) -> Vec<thrum_render::contribution::SceneDrawStage> {
        Vec::new()
    }

    fn rebuild(&mut self, _: &SceneSurface<'_>) -> Result<(), FrameError> {
        unreachable!("2D contribution")
    }

    fn rebuild_2d(&mut self, _: &SpriteContributionSurface<'_>) -> Result<(), FrameError> {
        Ok(())
    }

    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        *self.0.lock().expect("effect time") = Some(frame.time);
    }
}

#[test]
fn frame_host_samples_the_driver_at_each_display_target() {
    let Some(gpu) = thrum_test_support::gpu(
        "frame_host_samples_the_driver_at_each_display_target",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let OpenedDevice {
            device,
            queue,
            capabilities,
            ..
        } = DeviceRecipe::default()
            .open(&gpu.instance, None)
            .await
            .expect("Metal device");
        let extent = [16, 16];
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let surface = SurfaceProfile { extent, format };
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("driven world observer target"),
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
        let mut host = FrameHost::new_2d(
            &capabilities,
            &queue,
            FrameConfig {
                render_extent: extent,
                surface: Some(surface),
            },
            DrivenWorld::new(Script::default(), NonZeroU32::new(3).expect("cap"), TICK),
            Scene::default(),
            SpriteScene::new(
                EmbeddedSheet {
                    extent: [1, 1],
                    rgba: vec![255; 4].into(),
                    frames: vec![SpriteRect {
                        origin: [0, 0],
                        size: [1, 1],
                    }]
                    .into(),
                    texels_per_unit: 1.0,
                },
                1,
            ),
            SpriteSceneRequests::default(),
        )
        .await
        .expect("prepared two-dimensional host");
        host.set_timing_requested(false);

        let effect_time = Arc::new(Mutex::new(None));
        host.set_scene_contribution(Box::new(EffectObserver(Arc::clone(&effect_time))))
            .expect("observe the frame time delivered to effects");

        for (now, lead, tick, expected, phase, due) in [
            (5, 5, 1, 0.25, 0.25, Some(0)),
            (15, 5, 1, 0.5, 0.5, Some(0)),
            (25, 20, 1, 1.0, 1.125, Some(0)),
            (35, 50, 1, 1.0, 2.25, Some(15)),
            (45, 5, 3, 0.55, 0.55, Some(30)),
            (70, 20, 3, 1.0, 1.0, None),
            (80, 0, 4, 0.25, 0.25, Some(70)),
        ] {
            if now == 70 {
                host.world_mut().pause();
            } else if now == 80 {
                host.world_mut().resume();
            }
            host.set_presentation_lead(Duration::from_millis(lead));
            let report = host
                .drive(
                    Duration::from_millis(now),
                    Some(&SurfaceOutput {
                        view: &view,
                        extent,
                    }),
                )
                .expect("draw display sample");
            assert!(report.surface_presented);
            let sample = host.last_submitted_clock().expect("submitted phase");
            assert_eq!(sample.tick, tick);
            let (scene_tick, scene_alpha) = host.source().0.get().expect("sampled sprite view");
            assert_eq!(scene_tick, tick);
            assert!((scene_alpha - expected).abs() < f32::EPSILON);
            assert!((sample.presentation_alpha - expected).abs() < f32::EPSILON);
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("retire test draw");
            let frame = host
                .poll_completed()
                .expect("frame sample")
                .expect("retired draw");
            assert!((frame.display_phase - phase).abs() < f32::EPSILON);
            assert_eq!(frame.clock_interpolating, due.is_some());
            assert_eq!(frame.tick_due_time, due.map(Duration::from_millis));

            let time = effect_time
                .lock()
                .expect("effect time")
                .take()
                .expect("effect upload");
            assert_eq!(time.tick, tick);
            assert_eq!(time.tick_duration, TICK);
            assert!((time.alpha - expected).abs() < f32::EPSILON);
            assert!((time.display_phase - phase).abs() < f32::EPSILON);
            println!(
                "tick={tick} now={now}ms lead={lead}ms duration={:?} alpha={} phase={} due={:?}",
                time.tick_duration, time.alpha, time.display_phase, frame.tick_due_time
            );
        }
    });
}
