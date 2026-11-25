//! Device proof of driven-world frame sampling.
use std::{cell::Cell, convert::Infallible, num::NonZeroU32, time::Duration};

use thrum_core::{
    ClockSample, WorldEpoch,
    sim::{DrivenAdvance, DrivenWorld, SnapshotId, WorldDriver, WorldPublication},
};
use thrum_render::{
    capabilities::{DeviceRecipe, OpenedDevice},
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        FrameConfig, FrameError, FrameHost, SceneFrame, SceneSource, SpriteSceneRequests,
        SurfaceOutput,
    },
    sprite::{EmbeddedSheet, SpriteFrame, SpriteRect, SpriteScene, SpriteView},
    synthetic::FrameLighting,
};

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
            0 => (1, 0.125, 25.0),
            10 => (0, 0.375, 25.0),
            20 => (0, 0.625, 25.0),
            30 => (2, 0.5, 0.0),
            _ => unreachable!("script timestamp"),
        };
        self.0 += u64::from(completed_ticks);
        Ok(DrivenAdvance {
            completed_ticks,
            presentation: ClockSample::driven(now, fraction, rate).expect("phase"),
            next_due: Duration::from_millis(10),
        })
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
            DrivenWorld::new(Script::default(), NonZeroU32::new(3).expect("cap")),
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

        for (now, lead, tick, expected) in [
            (0, 5, 1, 0.25),
            (10, 5, 1, 0.5),
            (20, 20, 1, 1.0),
            (30, 5, 3, 0.5),
        ] {
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
            assert!(host.poll_completed().expect("frame sample").is_some());
        }
    });
}
