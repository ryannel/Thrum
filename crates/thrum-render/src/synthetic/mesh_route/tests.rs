//! The bucket route's Direct draw against the legacy route on the existing
//! cube input (root's old-vs-new Direct comparison), and its member lists
//! under activity churn.

use super::super::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight};
use super::super::{SyntheticScene, SyntheticView};
use crate::abi::InstanceRecord;
use crate::capabilities::DeviceCapabilities;
use crate::synthetic::mesh_arena::MeshId;

const EXTENT: [u32; 2] = [320, 180];

struct Gpu {
    capabilities: DeviceCapabilities,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Gpu {
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
        Gpu {
            capabilities,
            device,
            queue,
        }
    })
}

impl Gpu {
    fn prepare(&self, scene: &SyntheticScene, meshes: Option<Vec<MeshId>>) -> PreparedSynthetic {
        pollster::block_on(async {
            match meshes {
                Some(slots) => {
                    PreparedSynthetic::prepare_selected(
                        &self.capabilities,
                        &self.queue,
                        scene,
                        slots,
                    )
                    .await
                }
                None => {
                    PreparedSynthetic::prepare_with_capabilities(
                        &self.capabilities,
                        &self.queue,
                        scene,
                        FrameLighting::Diagnostic(SyntheticLight::default()),
                        1.0,
                    )
                    .await
                }
            }
        })
        .expect("prepare")
    }

    /// One frame of `scene` (whose instances `prepared` already holds).
    fn render(&self, prepared: &mut PreparedSynthetic, scene: &SyntheticScene) -> Vec<u8> {
        let mut target =
            OffscreenTarget::new(&self.device, prepared, EXTENT[0], EXTENT[1]).expect("target");
        prepared
            .update_frame(
                &self.queue,
                &mut target,
                scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            )
            .expect("update");
        let mut readback = target.readback(&self.device);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode_with_readback(&mut encoder, 41, &target, None, &mut readback)
            .expect("encode");
        self.queue.submit([encoder.finish()]);
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .expect("poll");
        readback
            .try_collect(41)
            .expect("collect")
            .expect("submitted")
    }

    /// Legacy and bucket-route Direct pixels for one scene, every slot the cube.
    fn both(&self, scene: &SyntheticScene) -> (Vec<u8>, Vec<u8>) {
        let mut legacy = self.prepare(scene, None);
        let mut route = self.prepare(scene, Some(vec![MeshId::CUBE; scene.instances.len()]));
        (
            self.render(&mut legacy, scene),
            self.render(&mut route, scene),
        )
    }
}

fn differing(a: &[u8], b: &[u8]) -> usize {
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(x, y)| x != y)
        .count()
}

fn perspective() -> SyntheticView {
    SyntheticView::perspective(
        [0.3, 0.6, -1.6],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        0.9,
        16.0 / 9.0,
        0.05,
        10.0,
    )
    .expect("view")
}

fn place(instance: &mut InstanceRecord, half: [f32; 3], center: [f32; 3], yaw: f32) {
    let (sin, cos) = yaw.sin_cos();
    let axes = [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]];
    for (axis, extent) in half.into_iter().enumerate() {
        for (row, &unit) in axes[axis].iter().enumerate() {
            instance.current[axis][row] = unit * extent;
            instance.normal[axis][row] = unit / extent;
        }
    }
    instance.current[3] = [center[0], center[1], center[2], 1.0];
    instance.previous = instance.current;
}

#[test]
fn one_ordinary_bucket_draws_the_legacy_image() {
    let gpu = gpu();
    for view in [None, Some(perspective())] {
        let mut scene = SyntheticScene::grid(400).expect("grid");
        if let Some(view) = view {
            scene.view = view;
        }
        let (legacy, route) = gpu.both(&scene);
        assert!(
            legacy.chunks(4).any(|p| p != &legacy[..4]),
            "the grid draws"
        );
        assert_eq!(
            differing(&legacy, &route),
            0,
            "orthographic: {}",
            view.is_none()
        );
    }
}

/// Root's old-vs-new Direct comparison where the classes differ: mirrored
/// boxes, and an equal-depth pair of an ordinary and a mirrored box in both
/// slot orders. The legacy route draws the whole scene two-sided in physical
/// order; the bucket route draws ordinary before mirrored with their own
/// pipelines. Differences are recorded, not judged.
#[test]
fn mirrored_classes_against_the_legacy_route() {
    let gpu = gpu();
    for (name, mirrored_first) in [("ordinary first", false), ("mirrored first", true)] {
        let mut scene = SyntheticScene::grid(4).expect("grid");
        scene.view = perspective();
        let mirror = |x: f32| [-0.25 * x, 0.25, 0.25];
        let (a, b) = if mirrored_first { (0, 1) } else { (1, 0) };
        // An equal-depth pair: the same box, one mirrored (its x axis negated
        // keeps the same surface), with different materials.
        place(&mut scene.instances[a], mirror(1.0), [-0.3, 0.0, 0.0], 0.4);
        place(&mut scene.instances[b], [0.25; 3], [-0.3, 0.0, 0.0], 0.4);
        scene.instances[a].ids[1] = 0;
        scene.instances[b].ids[1] = 1;
        // A lone mirrored box and a lone ordinary box.
        place(&mut scene.instances[2], mirror(1.0), [0.35, 0.0, 0.1], -0.7);
        place(&mut scene.instances[3], [0.2; 3], [0.0, -0.4, 0.3], 0.2);
        let (legacy, route) = gpu.both(&scene);
        eprintln!(
            "mirrored classes, {name}: {} of {} pixels differ between legacy and bucket Direct",
            differing(&legacy, &route),
            EXTENT[0] * EXTENT[1]
        );
    }
}

#[test]
fn members_follow_activity_churn_like_the_legacy_route() {
    let gpu = gpu();
    let mut scene = SyntheticScene::grid(256).expect("grid");
    scene.view = perspective();
    let mut legacy = gpu.prepare(&scene, None);
    let mut route = gpu.prepare(&scene, Some(vec![MeshId::CUBE; scene.instances.len()]));
    for frame in 0..4_u32 {
        let changed: Vec<u32> = (0..256).filter(|slot| (slot + frame) % 5 == 0).collect();
        for &slot in &changed {
            let instance = &mut scene.instances[slot as usize];
            instance.ids[3] = u32::from(instance.ids[3] == 0);
        }
        for prepared in [&mut legacy, &mut route] {
            prepared
                .upload_instances(&gpu.queue, &scene.instances, &changed)
                .expect("upload");
        }
        let legacy_pixels = gpu.render(&mut legacy, &scene);
        let route_pixels = gpu.render(&mut route, &scene);
        assert_eq!(differing(&legacy_pixels, &route_pixels), 0, "frame {frame}");
    }
}
