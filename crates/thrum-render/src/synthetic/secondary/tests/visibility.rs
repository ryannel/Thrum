//! A requested view draws an object behind the main camera, and its own
//! clip/least-size tests compact it without touching main submissions.
use super::*;
use crate::synthetic::{CullMode, FrameLighting};

fn count(device: &wgpu::Device, queue: &wgpu::Queue, args: &wgpu::Buffer) -> u32 {
    let bytes = thrum_test_support::readback::buffer(device, queue, args, ..20);
    u32::from_le_bytes(bytes[4..8].try_into().expect("count"))
}

#[cfg(target_os = "macos")]
#[test]
fn secondary_visibility_is_its_own_frustum_even_when_main_uses_occlusion() {
    let Some(crate::test_device::Opened {
        gpu, device, queue, ..
    }) = crate::test_device::open(
        "secondary_visibility_is_its_own_frustum_even_when_main_uses_occlusion",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::synthetic::tests::support::NO_FEATURES,
    )
    else {
        return;
    };
    pollster::block_on(async {
        let adapter = gpu.adapter;
        let capabilities = crate::capabilities::DeviceCapabilities::capture(&adapter, &device, 1);
        let camera = |height: f32| {
            SyntheticView::perspective(
                [0.0, height, -2.0],
                [0.0, height + 1.0, if height > 0.0 { -3.0 } else { -0.75 }],
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                0.1,
                20.0,
            )
            .expect("camera")
        };
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = camera(2.0);
        scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
        scene.materials[0].emissive_cutoff = [1.0, 0.5, 0.25, 0.0];
        for (bucket, mode) in [false, true]
            .into_iter()
            .flat_map(|bucket| [CullMode::Frustum, CullMode::Occlusion].map(|mode| (bucket, mode)))
        {
            scene.meshes = bucket.then(|| {
                Arc::new(crate::synthetic::SyntheticMeshes {
                    updates: None,
                    meshes: Vec::new(),
                    slots: vec![crate::synthetic::SyntheticMeshChoice::Cube],
                })
            });
            scene.camera_culling = mode;
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            )
            .await
            .expect("scene");
            assert_eq!(
                prepared.camera_culling(),
                mode,
                "real adapter admits requested mode"
            );
            prepared
                .prepare_secondary_views(
                    &queue,
                    &[SecondaryViewSlot::new(1.0).expect("slot")],
                    [64, 64],
                    &[],
                )
                .expect("views");
            prepared.set_contribution_passes(ContributionPasses {
                upload: true,
                compute: false,
            });
            let target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("target");
            let request = SecondaryViewRequest::new(&camera(-2.0), [64, 64], None, ViewAir::Camera)
                .expect("request");
            let mut asker = Asker {
                slot: prepared.secondary_handles[0].clone(),
                request: Some(request),
            };
            let (pixels, main) = render(&device, &queue, &prepared, &target, &mut asker);
            assert!(pixels.iter().filter(|p| p[3] > 0.0).count() > 100);
            let args = prepared.secondary[0]
                .culling
                .as_ref()
                .expect("view culling")
                .1
                .args();
            assert_eq!(count(&device, &queue, args), 1);
            asker.request = Some(request.with_least_pixels(1000.0).expect("size"));
            let (pixels, other_main) = render(&device, &queue, &prepared, &target, &mut asker);
            assert!(pixels.iter().all(|p| p[3] == 0.0));
            assert_eq!(
                count(&device, &queue, args),
                0,
                "least size changes the GPU list"
            );
            assert_eq!(main, other_main);
            asker.request = Some(
                SecondaryViewRequest::new(
                    &camera(-2.0),
                    [64, 64],
                    Some(
                        crate::contribution::ViewClipPlane::new([1.0, 0.0, 0.0, -10.0])
                            .expect("clip"),
                    ),
                    ViewAir::Camera,
                )
                .expect("request"),
            );
            let (pixels, other_main) = render(&device, &queue, &prepared, &target, &mut asker);
            assert!(pixels.iter().all(|p| p[3] == 0.0));
            assert_eq!(
                count(&device, &queue, args),
                0,
                "clip plane changes the GPU list"
            );
            assert_eq!(main, other_main);
        }
    });
}
