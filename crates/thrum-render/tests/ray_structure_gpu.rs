//! Geometry and membership answers from the submitted scene structure.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(feature = "creation-counters")]
mod metal {
    #[cfg(target_os = "macos")]
    use super::ray_support::support;

    #[cfg(target_os = "macos")]
    use std::sync::Arc;
    #[cfg(target_os = "macos")]
    use thrum_render::abi::InstanceRecord;
    #[cfg(target_os = "macos")]
    use thrum_render::synthetic::{SyntheticMeshChoice, SyntheticMeshes, SyntheticScene};

    thrum_test_support::macos_gpu_test! {
    fn each_active_mesh_hits_and_holes_miss() {
        let Some(gpu) = support::gate("ray structure membership") else {
            return;
        };
        let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
        let mut scene = SyntheticScene::grid(5).expect("scene");
        scene.ray_structure = true;
        for (slot, instance) in scene.instances.iter_mut().enumerate() {
            *instance = InstanceRecord::identity(u32::try_from(slot).expect("slot fits u32"), 0);
            instance.current[3][0] = slot as f32 * 20.0;
        }
        // Both rays take the middle of a triangle. Dropping the mirrored instance
        // or its sign moves its triangle to the opposite side of the slot.
        scene.instances[1].current[0][0] = -1.0;
        scene.instances[1].normal[0][0] = -1.0;
        // Scaling moves the local x=2 centre to x=6 around the slot origin.
        scene.instances[2].current[0][0] = 3.0;
        scene.instances[2].normal[0][0] = 1.0 / 3.0;
        scene.instances[3].deactivate();
        for instance in &mut scene.instances {
            instance.previous = instance.current;
        }
        scene.meshes = Some(Arc::new(SyntheticMeshes {
            meshes: vec![support::plane(2.0), support::plane(-2.0)],
            slots: vec![
                SyntheticMeshChoice::Cube,
                SyntheticMeshChoice::Mesh(0),
                SyntheticMeshChoice::Mesh(0),
                SyntheticMeshChoice::Cube,
                SyntheticMeshChoice::Mesh(1),
            ],
            updates: None,
        }));
        let (mut prepared, target) = support::prepare(&device, &queue, &caps, &scene);
        assert_eq!(prepared.ray_mesh_structure_count(), 4);
        let probe = support::Probe::new(&device, &prepared);
        let origins = [0.2, 18.0, 46.0, 60.2, 78.0, 95.0].map(|x| [x, 0.13, -4.0, 0.0]);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode(&mut encoder, caps.generation(), &target, None)
            .expect("draw and build");
        let readback = probe.record(&device, &mut encoder, &origins);
        let submission = queue.submit([encoder.finish()]);
        prepared.commit_frame();
        assert_eq!(support::answers(&device, readback, submission), [1, 2, 3, 0, 5, 0]);
    }
    }
}
