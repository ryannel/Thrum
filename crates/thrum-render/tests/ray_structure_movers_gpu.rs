//! Submitted poses and ray answers for the moving 10,000-slot scene.
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
    use thrum_render::synthetic::{
        FrameLighting, SyntheticMeshChoice, SyntheticMeshes, SyntheticScene,
    };

    #[cfg(target_os = "macos")]
    const MOVERS: usize = 1500;
    #[cfg(target_os = "macos")]
    const SLOTS: u32 = 10_000;

    thrum_test_support::macos_gpu_test! {
    fn every_record_matches_the_submitted_interpolated_pose() {
        let Some(gpu) = support::gate("ray structure movers") else {
            return;
        };
        let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
        let mut scene = SyntheticScene::grid(SLOTS).expect("scene");
        scene.ray_structure = true;
        for (slot, instance) in scene.instances.iter_mut().enumerate() {
            *instance = InstanceRecord::identity(u32::try_from(slot).expect("slot fits u32"), 0);
            instance.current[3] = [slot as f32 * 4.0, 1.0, 0.0, 1.0];
            instance.previous = instance.current;
        }
        let mut meshes = SyntheticMeshes {
            meshes: vec![support::plane(2.0), support::plane(-2.0)],
            slots: vec![SyntheticMeshChoice::Cube; SLOTS as usize],
            updates: None,
        };
        meshes.slots[0] = SyntheticMeshChoice::Mesh(0);
        meshes.slots[631] = SyntheticMeshChoice::Mesh(1);
        scene.meshes = Some(Arc::new(meshes.clone()));
        let (mut prepared, mut target) = support::prepare(&device, &queue, &caps, &scene);
        let mut probe = support::Probe::new(&device, &prepared);
        let mut previous_hit = -1.0;
        for frame in 0..8 {
            let alpha = if frame % 2 == 0 { 0.25 } else { 0.75 };
            for instance in &mut scene.instances[..MOVERS] {
                instance.previous = instance.current;
                instance.current[3][1] += 1.0;
            }
            scene.instances[0].previous[3] = [-100.0 - frame as f32 * 10.0, 1.0, 0.0, 1.0];
            scene.instances[0].current[3] = [-110.0 - frame as f32 * 10.0, 1.0, 0.0, 1.0];
            scene.instances[9000].set_generation(u32::from(frame % 2 == 0));
            scene.instances[9001].set_generation(u32::from(frame % 2 != 0));
            if frame == 4 {
                // Slot mesh changes use the renderer's admitted reprepare path.
                meshes.slots[0] = SyntheticMeshChoice::Mesh(1);
                meshes.slots[631] = SyntheticMeshChoice::Cube;
                scene.meshes = Some(Arc::new(meshes.clone()));
                (prepared, target) = support::prepare(&device, &queue, &caps, &scene);
                probe = support::Probe::new(&device, &prepared);
            }
            prepared
                .upload_instances(
                    &queue,
                    &scene.instances,
                    &(0..u32::try_from(MOVERS).expect("mover count fits u32"))
                        .chain([9000, 9001])
                        .collect::<Vec<_>>(),
                )
                .expect("upload");
            prepared
                .update_frame(
                    &queue,
                    &mut target,
                    &scene,
                    FrameLighting::Diagnostic(thrum_render::synthetic::SyntheticLight::default()),
                    alpha,
                )
                .expect("view");
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared
                .encode(&mut encoder, caps.generation(), &target, None)
                .expect("draw and build");
            let slot = &scene.instances[0];
            let centre = if frame < 4 { 2.0 } else { -2.0 };
            let hit =
                slot.previous[3][0] + alpha * (slot.current[3][0] - slot.previous[3][0]) + centre;
            let origins =
                [hit, previous_hit, slot.current[3][0] + centre].map(|x| [x, 1.1, -4.0, 0.0]);
            let readback = probe.record(&device, &mut encoder, &origins);
            queue.submit([encoder.finish()]);
            prepared.commit_frame();
            assert_eq!(
                support::answers(&device, &readback),
                [1, 0, 0],
                "frame {frame}"
            );
            let mut rows = prepared.ray_structure_instances().expect("recorded list");
            rows.sort_unstable_by_key(|row| row.slot);
            let expected = scene.instances.iter().filter(|slot| slot.is_active());
            assert_eq!(rows.len(), expected.clone().count(), "frame {frame}");
            for (row, slot) in rows.iter().zip(expected) {
                let mesh = match meshes.slots[slot.logical_slot() as usize] {
                    SyntheticMeshChoice::Cube => 0,
                    SyntheticMeshChoice::Mesh(index) => 2 + index,
                };
                assert_eq!((row.slot, row.mesh_row, row.mask), (slot.logical_slot(), mesh, 3));
                let mut model = slot.current;
                for (position, previous) in model[3][..3].iter_mut().zip(slot.previous[3]) {
                    *position = previous + alpha * (*position - previous);
                }
                for axis in 0..3 {
                    assert_eq!(
                        &row.transform[axis * 4..axis * 4 + 4],
                        &model.map(|column| column[axis])
                    );
                }
            }
            previous_hit = hit;
        }
    }
    }
}
