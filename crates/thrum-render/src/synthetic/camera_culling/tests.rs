//! The grazing grid view through the prepared scene: occlusion really hides
//! slots, the early phase replays last frame's set, and a still frame lists
//! nothing late. Pixel equality with Direct is in `tests/visibility.rs`.

use super::super::{FrameLighting, SyntheticLight, SyntheticScene, SyntheticView};
use super::*;
use crate::capabilities::DeviceCapabilities;

/// Occlusion only: the late phase's indirect arguments, for the Metal host
/// tests.
#[cfg(target_os = "macos")]
pub(crate) fn late_args(target: &OffscreenTarget) -> Option<&wgpu::Buffer> {
    target
        .occlusion
        .as_ref()
        .map(|late| late.target.late().args())
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
        let view = SyntheticView::perspective(
            [-0.2, 0.05, 0.6],
            [1.0, 0.0, 0.45],
            [0.0, 1.0, 0.0],
            1.0,
            16.0 / 9.0,
            0.01,
            10.0,
        )
        .expect("view");
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
                let first = target.visibility.as_ref().expect("culled");
                let late = target.occlusion.as_ref().map(|late| late.target.late());
                frames.push((
                    instances(&device, &queue, first.args()),
                    late.map(|late| instances(&device, &queue, late.args())),
                ));
            }
            counts.push(frames);
        }
        let framed = counts[0][0].0;
        let phases: Vec<(u32, u32)> = counts[1]
            .iter()
            .map(|&(early, late)| (early, late.expect("late")))
            .collect();
        // Frame 0 has no history, so its pyramid holds only the backdrop.
        assert_eq!(phases[0], (0, framed), "a cold frame draws everything late");
        // Frame 1 replays all of it early; its pyramid now holds the grid.
        assert_eq!(
            phases[1],
            (framed, 0),
            "the early phase replays last frame's set"
        );
        // From frame 2 the history holds only unoccluded slots.
        let [early, late] = [phases[2].0, phases[2].1];
        assert!(
            (early + late) * 10 < framed * 7,
            "occlusion hides at least 30%: {early} early + {late} late of {framed}: {phases:?}"
        );
        assert_eq!(
            phases[3],
            (early + late, 0),
            "a still view settles: {phases:?}"
        );
        // A capture from another camera between displays leaves the displayed
        // history: the next display frame replays it and adds nothing late.
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
    let late = target.occlusion.as_ref().map(|late| late.target.late());
    if let Some(late) = late {
        // The capture's own late phase did run and list slots.
        assert!(
            instances(device, queue, late.args()) > 0,
            "capture drew late"
        );
    }
}
