use std::sync::Arc;

use super::{EXTENT, Gpu, OffscreenTarget, place};
use crate::capabilities::DeviceCapabilities;
use crate::synthetic::{
    FrameLighting, PreparedSynthetic, SyntheticLight, SyntheticMeshChoice, SyntheticMeshes,
    SyntheticScene, SyntheticView, shadow,
};
use crate::visibility::CullMode;

fn scene() -> SyntheticScene {
    let mut scene = SyntheticScene::grid(3).expect("three slots");
    scene.sun_shadows = true;
    scene.view = SyntheticView::perspective(
        [44.0, 85.0, 90.0],
        [0.0, 35.0, 0.0],
        [0.0, 1.0, 0.0],
        1.0,
        16.0 / 9.0,
        1.0,
        400.0,
    )
    .expect("view");
    // The ground and standing tower bound the height/depth fit with or
    // without the pyramid. The pyramid floats well clear of both.
    place(
        &mut scene.instances[0],
        [112.0, 1.0, 112.0],
        [0.0, -1.0, 40.0],
        0.0,
    );
    place(
        &mut scene.instances[1],
        [16.0, 41.0, 16.0],
        [-45.0, 41.0, 90.0],
        0.0,
    );
    place(
        &mut scene.instances[2],
        [24.0, 6.0, 24.0],
        [44.0, 45.0, 90.0],
        0.0,
    );
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        meshes: vec![super::casters::pyramid()],
        slots: vec![
            SyntheticMeshChoice::Cube,
            SyntheticMeshChoice::Cube,
            SyntheticMeshChoice::Mesh(0),
        ],
        updates: None,
    }));
    scene
}

fn draw_layers(gpu: &Gpu, scene: &SyntheticScene) -> OffscreenTarget {
    let light = FrameLighting::Diagnostic(SyntheticLight {
        direction_to_light: [0.0, 1.0, 0.0],
        irradiance: [4.0; 3],
    });
    let mut prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
        &gpu.capabilities,
        &gpu.queue,
        scene,
        light,
        1.0,
    ))
    .expect("prepare");
    assert!(prepared.bucket_route.is_some(), "bucket route");
    assert_eq!(prepared.camera_culling(), scene.camera_culling);
    let mut target =
        OffscreenTarget::new(&gpu.device, &prepared, EXTENT[0], EXTENT[1]).expect("target");
    prepared
        .update_frame(&gpu.queue, &mut target, scene, light, 1.0)
        .expect("update");
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, gpu.capabilities.generation(), &target, None)
        .expect("draw scene");
    gpu.queue.submit([encoder.finish()]);
    target
}

#[test]
fn sun_cascades_gpu_aloft_casters_draw_into_cascades_only() {
    let backend = cfg!(target_os = "macos").then_some(wgpu::Backend::Metal);
    let Some(crate::test_device::Opened {
        gpu: admitted,
        device,
        queue,
        ..
    }) = crate::test_device::open(
        "aloft casters",
        backend,
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction::default(),
    )
    else {
        return;
    };
    let gpu = Gpu {
        capabilities: DeviceCapabilities::capture(&admitted.adapter, &device, 41),
        device,
        queue,
    };
    eprintln!(
        "{} ({:?})",
        gpu.capabilities.adapter().name,
        gpu.capabilities.adapter().backend
    );
    for mode in [CullMode::Direct, CullMode::Frustum, CullMode::Occlusion] {
        let mut scene = scene();
        scene.camera_culling = mode;
        let unmarked = draw_layers(&gpu, &scene);
        scene.aloft_mesh_rows = vec![0];
        let marked = draw_layers(&gpu, &scene);
        scene.aloft_mesh_rows.clear();
        scene.instances[2].deactivate();
        let absent = draw_layers(&gpu, &scene);
        let control = gpu.depth_differences(&unmarked, &absent);
        let marked_against_unmarked = gpu.depth_differences(&marked, &unmarked);
        let marked_against_absent = gpu.depth_differences(&marked, &absent);
        for (layer, control_changes) in control.into_iter().enumerate() {
            assert!(
                control_changes > 0,
                "{mode:?} layer {layer}: unmarked caster must draw"
            );
            let differences = if layer == shadow::HEIGHT_LAYER {
                marked_against_absent[layer]
            } else {
                marked_against_unmarked[layer]
            };
            assert_eq!(
                differences, 0,
                "{mode:?} layer {layer}: depth texels differ from the expected run"
            );
            eprintln!(
                "{mode:?} layer {layer}: unmarked caster changes {control_changes} texels; marked matches expected run"
            );
        }
    }
}
