//! The grazing grid view through the prepared scene: occlusion really hides
//! slots, the prepass replays last frame's visible set, and a still frame
//! settles. Pixel equality with Direct is in `tests/visibility.rs`.

use super::super::{FrameLighting, SyntheticLight, SyntheticScene, SyntheticView};
use super::*;
use crate::capabilities::DeviceCapabilities;

/// Occlusion only: the prepass list's indirect arguments, for the Metal
/// host tests.
#[cfg(target_os = "macos")]
pub(crate) fn prepass_args(target: &OffscreenTarget) -> Option<&wgpu::Buffer> {
    target
        .occlusion
        .as_ref()
        .map(|prepass| prepass.target.prepass().args())
}

fn instances(device: &wgpu::Device, queue: &wgpu::Queue, args: &wgpu::Buffer) -> u32 {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("culling test args"),
        size: visibility::ARGS_BYTES,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(args, 0, &staging, 0, visibility::ARGS_BYTES);
    queue.submit([encoder.finish()]);
    staging.slice(..).map_async(wgpu::MapMode::Read, |result| {
        result.expect("map");
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .expect("poll");
    let bytes = staging.slice(..).get_mapped_range().expect("mapped");
    u32::from_le_bytes(bytes[4..8].try_into().expect("instance count"))
}

/// Slot 0 becomes a wall across the grazing view, in front of most of the grid.
fn wall(scene: &mut SyntheticScene) {
    let wall = &mut scene.instances[0];
    let half = [0.005, 0.3, 0.2];
    for (axis, extent) in half.into_iter().enumerate() {
        for row in 0..3 {
            let unit = if axis == row { 1.0 } else { 0.0 };
            wall.current[axis][row] = unit * extent;
            wall.normal[axis][row] = unit / extent;
        }
    }
    wall.current[3] = [0.0, 0.05, 0.55, 1.0];
    wall.previous = wall.current;
}

#[test]
fn the_grazing_view_hides_slots_and_replays_its_visible_set() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 41);
        let view = grazing();
        let mut counts = Vec::new();
        for mode in [CullMode::Frustum, CullMode::Occlusion] {
            let mut scene = SyntheticScene::grid(10_000).expect("grid");
            scene.view = view;
            scene.camera_culling = mode;
            wall(&mut scene);
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            )
            .await
            .expect("prepare");
            let mut target = OffscreenTarget::new(&device, &prepared, 320, 180).expect("target");
            let mut frames = Vec::new();
            for frame in 0..5 {
                if frame == 4 {
                    capture(&device, &queue, &mut prepared, &scene);
                }
                prepared
                    .update_frame(
                        &queue,
                        &mut target,
                        &scene,
                        FrameLighting::Diagnostic(SyntheticLight::default()),
                        1.0,
                    )
                    .expect("update");
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                prepared
                    .encode(&mut encoder, 41, &target, None)
                    .expect("encode");
                queue.submit([encoder.finish()]);
                prepared.commit_frame();
                let drawn = target.visibility.as_ref().expect("culled");
                let prepass = target
                    .occlusion
                    .as_ref()
                    .map(|prepass| prepass.target.prepass());
                frames.push((
                    instances(&device, &queue, drawn.args()),
                    prepass.map(|prepass| instances(&device, &queue, prepass.args())),
                ));
            }
            counts.push(frames);
        }
        let framed = counts[0][0].0;
        // (prepass, final) per Occlusion frame.
        let phases: Vec<(u32, u32)> = counts[1]
            .iter()
            .map(|&(drawn, prepass)| (prepass.expect("prepass"), drawn))
            .collect();
        // Frame 0 has no history: an empty prepass and a far pyramid, so the
        // final list is the frustum set.
        assert_eq!(phases[0], (0, framed), "a cold frame draws the frustum set");
        // Frame 1's prepass replays all of it, so its pyramid holds the grid.
        let visible = phases[1].1;
        assert_eq!(phases[1].0, framed, "the prepass replays last frame's set");
        assert!(
            visible * 10 < framed * 7,
            "occlusion hides at least 30%: {visible} of {framed}: {phases:?}"
        );
        // From frame 2 the prepass holds only that set, whose depth is the
        // whole visible depth, so a still view settles.
        assert_eq!(phases[2], (visible, visible), "settles: {phases:?}");
        assert_eq!(phases[3], phases[2], "a still view stays: {phases:?}");
        // A capture from another camera between displays leaves the displayed
        // history: the next display frame replays it.
        assert_eq!(phases[4], phases[3], "capture kept the history: {phases:?}");
    });
}

/// A capture as `FrameHost::begin_capture` records one: a fresh target, the
/// current endpoint from another camera, submitted and never committed.
fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    scene: &SyntheticScene,
) {
    let mut scene = scene.clone();
    scene.view = SyntheticView::perspective(
        [0.4, 0.3, 0.1],
        [0.0, 0.0, 0.9],
        [0.0, 1.0, 0.0],
        0.9,
        16.0 / 9.0,
        0.01,
        10.0,
    )
    .expect("capture view");
    let mut target = OffscreenTarget::new(device, prepared, 320, 180).expect("capture target");
    prepared
        .update_frame(
            queue,
            &mut target,
            &scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            1.0,
        )
        .expect("capture update");
    let mut readback = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_with_readback(&mut encoder, 41, &target, None, &mut readback)
        .expect("capture encode");
    queue.submit([encoder.finish()]);
    if target.occlusion.is_some() {
        // The capture's own final cull did run and list slots.
        let drawn = target.visibility.as_ref().expect("culled");
        assert!(
            instances(device, queue, drawn.args()) > 0,
            "capture drew its final list"
        );
    }
}

/// The grazing grid view: the wall hides most of the grid behind it.
fn grazing() -> SyntheticView {
    SyntheticView::perspective(
        [-0.2, 0.05, 0.6],
        [1.0, 0.0, 0.45],
        [0.0, 1.0, 0.0],
        1.0,
        16.0 / 9.0,
        0.01,
        10.0,
    )
    .expect("view")
}

const PARITY: [u32; 2] = [320, 180];

/// One draw of the prepass list into a fresh cleared depth, through the
/// depth-only pipeline or the colour forward pipeline, read back as bytes.
fn list_depth(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
    depth_only: bool,
) -> Vec<u8> {
    let size = wgpu::Extent3d {
        width: PARITY[0],
        height: PARITY[1],
        depth_or_array_layers: 1,
    };
    let texture = |format, label| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    };
    let depth = texture(DEPTH_FORMAT, "parity depth");
    let scene = texture(super::super::SCENE_FORMAT, "parity scene");
    let motion = texture(super::super::MOTION_FORMAT, "parity motion");
    let [depth_view, scene_view, motion_view] = [&depth, &scene, &motion]
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
    let color = |view| {
        Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Discard,
            },
        })
    };
    let colors = if depth_only {
        Vec::new()
    } else {
        vec![color(&scene_view), color(&motion_view)]
    };
    let prepass = target.occlusion.as_ref().expect("occlusion");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("parity"),
            color_attachments: &colors,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        prepared
            .bind_forward(&mut pass, target, &prepass.view_bind_group, 0, depth_only)
            .expect("bind");
        pass.draw_indexed_indirect(prepass.target.prepass().args(), 0);
    }
    let row = PARITY[0] * 4;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("parity readback"),
        size: u64::from(row * PARITY[1]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &depth,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::DepthOnly,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    staging
        .slice(..)
        .get_mapped_range()
        .expect("mapped")
        .to_vec()
}

/// Prepass parity (design, *Prepass parity*): the depth-only pipeline writes
/// byte for byte the depth the colour forward pipeline writes for the same
/// list, view group, index buffer and extent, for the one-sided and the
/// two-sided split.
#[test]
fn the_depth_prepass_writes_the_forward_depth_of_the_same_list() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 41);
        for two_sided in [false, true] {
            let mut scene = SyntheticScene::grid(10_000).expect("grid");
            scene.view = grazing();
            scene.camera_culling = CullMode::Occlusion;
            wall(&mut scene);
            if two_sided {
                // One mirrored slot puts the whole list on the two-sided pipelines.
                let mirrored = &mut scene.instances[1];
                for row in 0..3 {
                    mirrored.current[2][row] = -mirrored.current[2][row];
                    mirrored.normal[2][row] = -mirrored.normal[2][row];
                }
                mirrored.previous = mirrored.current;
            }
            let lighting = FrameLighting::Diagnostic(SyntheticLight::default());
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                lighting,
                1.0,
            )
            .await
            .expect("prepare");
            assert_eq!(prepared.instance_upload.two_sided_count() != 0, two_sided);
            let mut target =
                OffscreenTarget::new(&device, &prepared, PARITY[0], PARITY[1]).expect("target");
            // Two frames: the second prepass list replays the first's set.
            for _ in 0..2 {
                prepared
                    .update_frame(&queue, &mut target, &scene, lighting, 1.0)
                    .expect("update");
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                prepared
                    .encode(&mut encoder, 41, &target, None)
                    .expect("encode");
                queue.submit([encoder.finish()]);
                prepared.commit_frame();
            }
            let prepass = list_depth(&device, &queue, &prepared, &target, true);
            let forward = list_depth(&device, &queue, &prepared, &target, false);
            let (prepass, forward) = (prepass.as_chunks::<4>().0, forward.as_chunks::<4>().0);
            let covered = prepass.iter().filter(|&&texel| texel != [0; 4]).count();
            assert!(
                covered * 4 > prepass.len(),
                "the list covers over a quarter of the view: {covered}"
            );
            let differing = prepass.iter().zip(forward).filter(|(a, b)| a != b).count();
            assert_eq!(differing, 0, "two-sided {two_sided}: texels differ");
        }
    });
}
