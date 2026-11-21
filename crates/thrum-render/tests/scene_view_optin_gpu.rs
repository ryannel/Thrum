//! A sibling that does not opt in receives neither view surfaces nor view callbacks.
mod support;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{
        ContributionFrame, ContributionPasses, SceneContribution, SceneContributions, SceneSurface,
        SecondaryShadowCascade, SecondaryViewFrame, SecondaryViewRequest, ViewAir,
    },
    synthetic::SyntheticScene,
};

#[derive(Default)]
struct Calls {
    prepared: AtomicUsize,
    uploaded: AtomicUsize,
    drawn: AtomicUsize,
}

struct MainOnly(Arc<Calls>);

impl SceneContribution for MainOnly {
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        assert!(surface.secondary_surfaces.is_empty());
        assert!(surface.secondary_views.is_empty());
        self.0.prepared.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
        self.0.uploaded.fetch_add(1, Ordering::Relaxed);
    }

    fn draw(&self, _: &mut wgpu::RenderPass<'_>) {
        self.0.drawn.fetch_add(1, Ordering::Relaxed);
    }

    fn encode_secondary_upload(
        &mut self,
        _: &mut wgpu::CommandEncoder,
        _: &ContributionFrame,
        _: &[SecondaryViewFrame],
    ) {
        panic!("main-only contribution received view upload");
    }

    fn draw_secondary(&self, _: usize, _: &mut wgpu::RenderPass<'_>) {
        panic!("main-only contribution received view draw");
    }

    fn draw_secondary_shadow(
        &self,
        _: usize,
        _: SecondaryShadowCascade,
        _: &mut wgpu::RenderPass<'_>,
    ) {
        panic!("main-only contribution received view Sun draw");
    }
}

#[test]
fn an_unopted_sibling_has_only_its_ordinary_pass() {
    let Some(gpu) = thrum_test_support::gpu("main-only contribution", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let view = support::camera(0.0, -2.0);
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = view;
        scene.sun_shadows = true;
        let mut host = support::host(&caps, &queue, scene.clone()).await;
        let base = Arc::new(Calls::default());
        let before = host.pipeline_creation_count();
        host.set_scene_contribution(Box::new(MainOnly(Arc::clone(&base))))
            .expect("main only");
        assert_eq!(before, host.pipeline_creation_count());
        let expected = host.capture_pixels().expect("ordinary frame");

        let mut host = support::host(&caps, &queue, scene).await;
        let (slot, control) = support::Observer::new(1);
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(&view, [64, 64], None, ViewAir::Camera).expect("request"),
        );
        host.set_scene_contribution(Box::new(slot))
            .expect("slot only");
        host.capture_pixels().expect("asked frame");
        let before = host.pipeline_creation_count();
        let (slot, control) = support::Observer::new(1);
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(&view, [64, 64], None, ViewAir::Camera).expect("request"),
        );
        let joined = Arc::new(Calls::default());
        host.set_scene_contribution(Box::new(SceneContributions::new(vec![
            Box::new(MainOnly(Arc::clone(&joined))),
            Box::new(slot),
        ])))
        .expect("main-only sibling and slot");
        let delta = host
            .pipeline_creation_count()
            .zip(before)
            .map(|(after, before)| after - before);
        // Repeat the same attachment transition without the main-only sibling.
        let before = host.pipeline_creation_count();
        assert_eq!(
            host.capture_pixels().expect("asked frame with sibling"),
            expected
        );
        let bytes = support::output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        assert!(
            support::pixel(&bytes, 32 * 64 + 32).1 > 0.0,
            "the slot recorded geometry"
        );
        for (ordinary, joined) in [
            (&base.prepared, &joined.prepared),
            (&base.uploaded, &joined.uploaded),
            (&base.drawn, &joined.drawn),
        ] {
            assert!(ordinary.load(Ordering::Relaxed) > 0);
            assert_eq!(
                ordinary.load(Ordering::Relaxed),
                joined.load(Ordering::Relaxed)
            );
        }
        let (slot, _) = support::Observer::new(1);
        host.set_scene_contribution(Box::new(slot))
            .expect("slot alone again");
        assert_eq!(
            delta,
            host.pipeline_creation_count()
                .zip(before)
                .map(|(after, before)| after - before)
        );
    });
}
