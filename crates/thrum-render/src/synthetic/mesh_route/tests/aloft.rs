use std::sync::Arc;
use std::time::Duration;

use super::{EXTENT, Gpu, OffscreenTarget, place};
use crate::capabilities::DeviceCapabilities;
use crate::synthetic::{
    FrameLighting, PreparedSynthetic, SyntheticLight, SyntheticMeshChoice, SyntheticMeshes,
    SyntheticScene, SyntheticView, shadow,
};
use crate::visibility::CullMode;

const ROW_BYTES: u32 = shadow::SIZE * 4;
const LAYER_BYTES: usize = (ROW_BYTES * shadow::SIZE) as usize;

fn scene() -> SyntheticScene {
    let mut scene = SyntheticScene::grid(3).expect("three slots");
    scene.sun_shadows = true;
    scene.view = SyntheticView::perspective(
        [0.0, 85.0, 170.0],
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

fn read_layers(gpu: &Gpu, scene: &SyntheticScene) -> Vec<u8> {
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
    let maps = target
        .textures
        .texture(target.ids.shadow.as_ref().expect("map resource"))
        .expect("Sun maps");
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Sun map layer readback"),
        size: u64::from(ROW_BYTES * shadow::SIZE * shadow::LAYERS),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, gpu.capabilities.generation(), &target, None)
        .expect("draw scene");
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: maps,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::DepthOnly,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ROW_BYTES),
                rows_per_image: Some(shadow::SIZE),
            },
        },
        wgpu::Extent3d {
            width: shadow::SIZE,
            height: shadow::SIZE,
            depth_or_array_layers: shadow::LAYERS,
        },
    );
    encoder.map_buffer_on_submit(&staging, wgpu::MapMode::Read, .., |result| {
        result.expect("map");
    });
    let submission = gpu.queue.submit([encoder.finish()]);
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })
        .expect("readback complete");
    staging
        .slice(..)
        .get_mapped_range()
        .expect("mapped depths")
        .to_vec()
}

#[test]
fn aloft_casters_draw_into_both_cascades_only() {
    let backend = cfg!(target_os = "macos").then_some(wgpu::Backend::Metal);
    let Some(admitted) = thrum_test_support::gpu("aloft casters", backend) else {
        return;
    };
    let (device, queue) = admitted.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
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
        let unmarked = read_layers(&gpu, &scene);
        scene.aloft_mesh_rows = vec![0];
        let marked = read_layers(&gpu, &scene);
        scene.aloft_mesh_rows.clear();
        scene.instances[2].deactivate();
        let absent = read_layers(&gpu, &scene);
        for (layer, ((marked, unmarked), absent)) in marked
            .as_chunks::<LAYER_BYTES>()
            .0
            .iter()
            .zip(unmarked.as_chunks::<LAYER_BYTES>().0)
            .zip(absent.as_chunks::<LAYER_BYTES>().0)
            .enumerate()
        {
            let control_changes = super::differing(unmarked, absent);
            assert!(
                control_changes > 0,
                "{mode:?} layer {layer}: unmarked caster must draw"
            );
            let expected = if layer == shadow::HEIGHT_LAYER {
                absent
            } else {
                unmarked
            };
            assert_eq!(
                super::differing(marked, expected),
                0,
                "{mode:?} layer {layer}: depth texels differ from the expected run"
            );
            eprintln!(
                "{mode:?} layer {layer}: unmarked caster changes {control_changes} texels; marked matches expected run"
            );
        }
    }
}
