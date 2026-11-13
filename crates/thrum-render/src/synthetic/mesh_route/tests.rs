//! The bucket route's Direct draw against the legacy route on the existing
//! cube input (root's old-vs-new Direct comparison), and its member lists
//! under activity churn.

use super::super::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight};
use super::super::{SyntheticScene, SyntheticView};
use crate::abi::InstanceRecord;
use crate::capabilities::DeviceCapabilities;
use crate::synthetic::mesh_arena::MeshId;

mod admission;
mod caster_cull;
mod casters;
mod grazing;

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
        self.render_counted(prepared, scene).0
    }

    /// One frame, and the culled route's drawn instances over all buckets.
    fn render_counted(
        &self,
        prepared: &mut PreparedSynthetic,
        scene: &SyntheticScene,
    ) -> (Vec<u8>, Option<u32>) {
        self.render_lit(
            prepared,
            scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
        )
    }

    /// One frame under `lighting`, and the culled route's drawn instances.
    fn render_lit(
        &self,
        prepared: &mut PreparedSynthetic,
        scene: &SyntheticScene,
        lighting: FrameLighting,
    ) -> (Vec<u8>, Option<u32>) {
        assert_eq!(
            prepared.camera_culling(),
            scene.camera_culling,
            "resolved mesh oracle mode"
        );
        let mut target =
            OffscreenTarget::new(&self.device, prepared, EXTENT[0], EXTENT[1]).expect("target");
        prepared
            .update_frame(&self.queue, &mut target, scene, lighting, 1.0)
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
        let pixels = readback
            .try_collect(41)
            .expect("collect")
            .expect("submitted");
        let drawn = target
            .visibility
            .as_ref()
            .map(|culled| self.drawn(culled.args()));
        (pixels, drawn)
    }

    /// Instances over every bucket's indirect argument record.
    fn drawn(&self, args: &wgpu::Buffer) -> u32 {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("route test args"),
            size: args.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(args, 0, &staging, 0, args.size());
        self.queue.submit([encoder.finish()]);
        staging.slice(..).map_async(wgpu::MapMode::Read, |result| {
            result.expect("map");
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .expect("poll");
        let bytes = staging.slice(..).get_mapped_range().expect("mapped");
        bytes
            .as_chunks::<20>()
            .0
            .iter()
            .map(|record| u32::from_le_bytes(record[4..8].try_into().expect("count")))
            .sum()
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

/// A mixed field: both arena rows, ordinary, mirrored and degenerate
/// (two-sided) instances, many outside the view.
fn mixed_field(count: u32) -> (SyntheticScene, Vec<MeshId>) {
    let mut scene = SyntheticScene::grid(count).expect("grid");
    scene.view = SyntheticView::perspective(
        [0.0, 0.25, -1.4],
        [0.2, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        0.6,
        16.0 / 9.0,
        0.05,
        10.0,
    )
    .expect("view");
    let side = (count - 1).isqrt() + 1;
    let mut meshes = Vec::with_capacity(count as usize);
    for slot in 0..count {
        let (x, z) = ((slot % side) as f32, (slot / side) as f32);
        let center = [x * 0.12 - 1.0, 0.0, z * 0.12 - 0.6];
        let half = match slot % 7 {
            0 => [-0.04, 0.04, 0.04],
            _ => [0.04; 3],
        };
        let instance = &mut scene.instances[slot as usize];
        place(instance, half, center, slot as f32 * 0.37);
        if slot % 7 == 3 {
            // A sheared basis with parallel x and y axes: determinant zero,
            // so the two-sided class, with the box's own normals.
            instance.current[1] = instance.current[0].map(|value| value * 0.5);
            instance.previous = instance.current;
        }
        meshes.push(if slot % 3 == 0 {
            MeshId::CUBE_REVERSED
        } else {
            MeshId::CUBE
        });
    }
    (scene, meshes)
}

#[test]
fn frustum_buckets_match_bucket_direct_on_a_mixed_field() {
    let gpu = gpu();
    let (mut scene, meshes) = mixed_field(900);
    let mut direct = gpu.prepare(&scene, Some(meshes.clone()));
    scene.camera_culling = crate::visibility::CullMode::Frustum;
    let mut frustum = gpu.prepare(&scene, Some(meshes));
    for frame in 0..7_u32 {
        if frame > 0 {
            // Churn activity and class: toggle some slots, mirror others. The
            // last frame mirrors most of the field, moving hundreds of slots
            // between buckets so segments cross workgroup boundaries.
            let changed: Vec<u32> = if frame == 4 {
                (0..900)
                    .filter(|slot| slot % 7 != 0 && slot % 7 != 3)
                    .collect()
            } else if frame >= 5 {
                // Deactivate two thirds of the field, then reactivate it: the
                // member list loses workgroups and gains them back.
                (0..900).filter(|slot| slot % 3 != 0).collect()
            } else {
                (0..900)
                    .filter(|slot| (slot * 7 + frame) % 11 == 0)
                    .collect()
            };
            for &slot in &changed {
                let instance = &mut scene.instances[slot as usize];
                if frame >= 5 {
                    instance.ids[3] = u32::from(frame == 6);
                } else if slot % 2 == 0 && frame != 4 {
                    instance.ids[3] = u32::from(instance.ids[3] == 0);
                } else {
                    instance.current[0] = instance.current[0].map(|value| -value);
                    instance.normal[0] = instance.normal[0].map(|value| -value);
                    instance.previous = instance.current;
                }
            }
            for prepared in [&mut direct, &mut frustum] {
                prepared
                    .upload_instances(&gpu.queue, &scene.instances, &changed)
                    .expect("upload");
            }
        }
        let (reference, none) = gpu.render_counted(&mut direct, &scene);
        let (culled, drawn) = gpu.render_counted(&mut frustum, &scene);
        assert_eq!(none, None);
        let active = scene.instances.iter().filter(|i| i.ids[3] != 0).count();
        let drawn = drawn.expect("frustum counts") as usize;
        assert!(
            drawn > 50 && drawn < active,
            "frame {frame}: drew {drawn} of {active}"
        );
        eprintln!("frame {frame}: frustum buckets drew {drawn} of {active}");
        assert_eq!(differing(&reference, &culled), 0, "frame {frame}");
        // The public count sums every bucket's record.
        let mut target =
            OffscreenTarget::new(&gpu.device, &frustum, EXTENT[0], EXTENT[1]).expect("target");
        frustum
            .update_frame(
                &gpu.queue,
                &mut target,
                &scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            )
            .expect("update");
        let counted = frustum
            .submit_counted((&gpu.device, &gpu.queue), 41, &mut target, None)
            .expect("counted");
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(counted.submission().clone()),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .expect("poll");
        let counts = counted.take().expect("counts");
        assert_eq!(counts.drawn as usize, drawn, "frame {frame}");
    }
}

/// Every class in its own bucket, every slot the cube, no equal-depth
/// pairs: the bucket route reorders draws across classes and must still
/// draw the legacy two-sided image. This checks each bucket's members
/// against an independent route.
#[test]
fn several_class_buckets_draw_the_legacy_image_without_ties() {
    let gpu = gpu();
    let (scene, _) = mixed_field(900);
    let (legacy, route) = gpu.both(&scene);
    assert_eq!(differing(&legacy, &route), 0);
}

/// Prepared with most of the field inactive, then fully activated: the
/// member list gains workgroups the preparation never dispatched.
#[test]
fn frustum_buckets_follow_a_growing_member_list() {
    let gpu = gpu();
    let (mut scene, meshes) = mixed_field(900);
    for (slot, instance) in (0_u32..).zip(&mut scene.instances) {
        instance.ids[3] = u32::from(slot % 3 == 0);
    }
    let mut direct = gpu.prepare(&scene, Some(meshes.clone()));
    scene.camera_culling = crate::visibility::CullMode::Frustum;
    let mut frustum = gpu.prepare(&scene, Some(meshes));
    let changed: Vec<u32> = (0..900).filter(|slot| slot % 3 != 0).collect();
    for &slot in &changed {
        scene.instances[slot as usize].ids[3] = 1;
    }
    for prepared in [&mut direct, &mut frustum] {
        prepared
            .upload_instances(&gpu.queue, &scene.instances, &changed)
            .expect("upload");
    }
    let (reference, _) = gpu.render_counted(&mut direct, &scene);
    let (culled, drawn) = gpu.render_counted(&mut frustum, &scene);
    assert!(drawn.expect("counts") > 500);
    assert_eq!(differing(&reference, &culled), 0);
}
