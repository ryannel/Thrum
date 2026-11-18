//! Scene-local absence of the ray path, including byte-identical output.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod metal {
    use super::ray_support::support;

    use thrum_render::capabilities::RayStructureSelection;
    use thrum_render::synthetic::{OffscreenTarget, PreparedSynthetic, SyntheticScene};

    fn submit_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        prepared: &PreparedSynthetic,
        target: &OffscreenTarget,
    ) -> Vec<u8> {
        let generation = prepared.capabilities().generation();
        let mut readback = target.readback(device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode_with_readback(&mut encoder, generation, target, None, &mut readback)
            .expect("capture ordinary submitted bytes");
        queue.submit([encoder.finish()]);
        support::wait(device);
        readback
            .try_collect(generation)
            .expect("collect readback")
            .expect("completed")
    }

    #[test]
    fn no_demand_has_no_structure_and_does_not_change_pixels_or_usages() {
        let gpu = support::gate("ray structure absence");
        let (ordinary, ordinary_queue, ordinary_caps) = support::device(&gpu, "", 1);
        let (device, queue, caps) = support::device(&gpu, "ray-query", 2);
        let mut scene = SyntheticScene::grid(81).expect("scene");
        let (reference, reference_target) =
            support::prepare(&ordinary, &ordinary_queue, &ordinary_caps, &scene);
        let reference_bytes =
            submit_bytes(&ordinary, &ordinary_queue, &reference, &reference_target);
        // FNV-1a over the 64x64 SDR bytes captured before ray demand was added.
        // The retained record includes the same file's SHA-256 and its recipe.
        let fingerprint = reference_bytes
            .iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
            });
        assert_eq!(fingerprint, 0xfdda_b10d_7411_3860);
        let usages = reference.ray_structure_input_usages();
        assert!(
            usages
                .iter()
                .all(|usage| !usage.contains(wgpu::BufferUsages::BLAS_INPUT))
        );
        let names: Vec<_> = reference
            .graph(&reference_target, true)
            .expect("graph")
            .pass_names()
            .map(str::to_owned)
            .collect();

        let (unpaired, unpaired_target) = support::prepare(&device, &queue, &caps, &scene);
        scene.ray_structure = true;
        let (traced, traced_target) = support::prepare(&device, &queue, &caps, &scene);
        let (unsupported, unsupported_target) =
            support::prepare(&ordinary, &ordinary_queue, &ordinary_caps, &scene);
        scene.ray_structure = false;
        let (paired, paired_target) = support::prepare(&device, &queue, &caps, &scene);

        assert_eq!(
            traced.ray_structure_selection(),
            RayStructureSelection::Enabled
        );
        assert_eq!(traced.ray_mesh_structure_count(), 2);
        assert!(
            traced
                .graph(&traced_target, true)
                .expect("traced graph")
                .pass_names()
                .any(|name| name == "ray structure build")
        );
        assert_eq!(
            submit_bytes(&device, &queue, &traced, &traced_target),
            reference_bytes
        );

        for (device, queue, prepared, target, selection) in [
            (
                &ordinary,
                &ordinary_queue,
                &reference,
                &reference_target,
                RayStructureSelection::NotRequested,
            ),
            (
                &ordinary,
                &ordinary_queue,
                &unsupported,
                &unsupported_target,
                RayStructureSelection::MissingRayQuery,
            ),
            (
                &device,
                &queue,
                &unpaired,
                &unpaired_target,
                RayStructureSelection::NotRequested,
            ),
            (
                &device,
                &queue,
                &paired,
                &paired_target,
                RayStructureSelection::NotRequested,
            ),
        ] {
            assert_eq!(prepared.ray_structure_selection(), selection);
            assert_eq!(prepared.ray_mesh_structure_count(), 0);
            assert!(prepared.ray_structure_instances().is_none());
            assert_eq!(prepared.ray_structure_input_usages(), usages);
            assert_eq!(
                prepared
                    .graph(target, true)
                    .expect("graph")
                    .pass_names()
                    .collect::<Vec<_>>(),
                names
            );
            assert_eq!(
                submit_bytes(device, queue, prepared, target),
                reference_bytes
            );
        }
        // Exact reference images are independently pinned by exact-images on main.
        assert!(
            reference_bytes
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[..3] != reference_bytes[..3])
        );
        if let Some(path) = std::env::var_os("THRUM_REFERENCE_OUTPUT") {
            std::fs::write(path, reference_bytes).expect("write ordinary reference witness");
        }
    }

    #[test]
    fn portable_request_falls_back_before_allocation_even_when_device_enabled_rays() {
        let gpu = support::gate("portable ray structure fallback");
        let (ordinary, ordinary_queue, ordinary_caps) = support::device(&gpu, "", 1);
        let (device, queue, caps) = support::device(&gpu, "ray-query", 2);
        let ordinary_caps = ordinary_caps
            .with_recipe_override("portable")
            .expect("portable");
        let caps = caps.with_recipe_override("portable").expect("portable");
        let mut scene = SyntheticScene::grid(81).expect("scene");
        let (reference, target) =
            support::prepare(&ordinary, &ordinary_queue, &ordinary_caps, &scene);
        let bytes = submit_bytes(&ordinary, &ordinary_queue, &reference, &target);
        let usages = reference.ray_structure_input_usages();
        let names = reference.graph(&target, true).expect("graph");
        assert!(!names.pass_names().any(|name| name == "ray structure build"));
        assert!(
            usages
                .iter()
                .all(|usage| !usage.contains(wgpu::BufferUsages::BLAS_INPUT))
        );

        for (device, queue, caps) in [
            (&ordinary, &ordinary_queue, &ordinary_caps),
            (&device, &queue, &caps),
        ] {
            for (requested, selection) in [
                (false, RayStructureSelection::NotRequested),
                (true, RayStructureSelection::UnsupportedRecipe),
            ] {
                scene.ray_structure = requested;
                let (prepared, target) = support::prepare(device, queue, caps, &scene);
                assert_eq!(prepared.ray_structure_selection(), selection);
                assert_eq!(prepared.ray_mesh_structure_count(), 0);
                assert!(prepared.ray_structure_instances().is_none());
                assert_eq!(prepared.ray_structure_input_usages(), usages);
                assert!(
                    prepared
                        .graph(&target, true)
                        .expect("graph")
                        .pass_names()
                        .eq(names.pass_names())
                );
                assert_eq!(submit_bytes(device, queue, &prepared, &target), bytes);
            }
        }
    }
}
