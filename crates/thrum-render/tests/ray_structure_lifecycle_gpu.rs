//! First submission, discarded encoders and replacement devices on Metal.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(feature = "creation-counters")]
mod metal {
    #[cfg(target_os = "macos")]
    use super::ray_support::support;
    #[cfg(target_os = "macos")]
    use thrum_render::error::RenderError;

    #[cfg(target_os = "macos")]
    use std::sync::Arc;
    #[cfg(target_os = "macos")]
    use thrum_render::abi::InstanceRecord;
    #[cfg(target_os = "macos")]
    use thrum_render::synthetic::{
        FrameLighting, SyntheticMeshChoice, SyntheticMeshes, SyntheticScene,
    };

    thrum_test_support::macos_gpu_test! {
    fn discarded_first_scene_build_replays_and_replacement_uses_its_prepared_meshes() {
        let Some(gpu) = support::gate("ray structure lifecycle") else {
            return;
        };
        let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.ray_structure = true;
        scene.instances[0] = InstanceRecord::identity(0, 0);
        scene.meshes = Some(Arc::new(SyntheticMeshes {
            meshes: vec![support::plane(2.0)],
            slots: vec![SyntheticMeshChoice::Mesh(0)],
            updates: None,
        }));
        let (mut prepared, mut target) = support::prepare(&device, &queue, &caps, &scene);
        let mut discarded =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode(&mut discarded, 1, &target, None)
            .expect("recorded first TLAS over prepared meshes");
        drop(discarded.finish());

        scene.instances[0].current[3][0] = 8.0;
        prepared
            .upload_instances(&queue, &scene.instances, &[0])
            .expect("upload moved slot");
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Diagnostic(thrum_render::synthetic::SyntheticLight::default()),
                0.5,
            )
            .expect("interpolated frame");
        let origins = [2.0, 6.0, 10.0].map(|x| [x, 0.1, -4.0, 0.0]);
        let probe = support::Probe::new(&device, &prepared);
        let mut submitted =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode(&mut submitted, 1, &target, None)
            .expect("replayed first build");
        let readback = probe.record(&device, &mut submitted, &origins);
        let submission = queue.submit([submitted.finish()]);
        prepared.commit_frame();
        assert_eq!(support::answers(&device, readback, submission), [0, 1, 0]);

        let (replacement, new_queue, new_caps) = support::device(&gpu, "ray-query", 2);
        assert!(!caps.matches(&replacement));
        assert!(!new_caps.matches(&device));
        scene.instances[0].previous = scene.instances[0].current;
        let (new_prepared, new_target) =
            support::prepare(&replacement, &new_queue, &new_caps, &scene);
        let mut encoder =
            replacement.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for result in [
            prepared.encode(&mut encoder, 2, &target, None),
            new_prepared.encode(&mut encoder, 2, &target, None),
        ] {
            assert!(matches!(result, Err(RenderError::StaleTarget)));
        }
        let probe = support::Probe::new(&replacement, &new_prepared);
        new_prepared
            .encode(&mut encoder, 2, &new_target, None)
            .expect("first replacement frame");
        let readback = probe.record(&replacement, &mut encoder, &origins);
        let submission = new_queue.submit([encoder.finish()]);
        assert_eq!(support::answers(&replacement, readback, submission), [0, 0, 1]);
    }
    }
}
