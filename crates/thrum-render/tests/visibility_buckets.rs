//! Bucket-ordered Direct is the oracle at every submitted frame, including
//! stale history, class churn, physical compaction and resource replacement.
use std::{sync::Arc, time::Duration};
use thrum_render::{
    abi::InstanceRecord,
    capabilities::DeviceCapabilities,
    synthetic::{
        CullMode, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticMesh,
        SyntheticMeshChoice, SyntheticMeshes, SyntheticScene, SyntheticVertex, SyntheticView,
    },
};

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    caps: DeviceCapabilities,
}
impl Gpu {
    async fn new(generation: u64) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Metal adapter");
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let caps = DeviceCapabilities::capture(&adapter, &device, generation);
        Self {
            device,
            queue,
            caps,
        }
    }
    async fn prepare(&self, scene: &SyntheticScene, mode: CullMode, extent: [u32; 2]) -> Sequence {
        let mut scene = scene.clone();
        scene.camera_culling = mode;
        let prepared = PreparedSynthetic::prepare_with_capabilities(
            &self.caps,
            &self.queue,
            &scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            1.0,
        )
        .await
        .expect("mesh preparation");
        assert_eq!(prepared.camera_culling(), mode, "resolved mode");
        let target =
            OffscreenTarget::new(&self.device, &prepared, extent[0], extent[1]).expect("target");
        Sequence { prepared, target }
    }
    fn wait(&self) {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(20)),
            })
            .expect("GPU wait");
    }
}
struct Sequence {
    prepared: PreparedSynthetic,
    target: OffscreenTarget,
}
impl Sequence {
    fn frame(
        &mut self,
        gpu: &Gpu,
        scene: &SyntheticScene,
        changed: &[u32],
    ) -> (Vec<u8>, Option<u32>) {
        self.prepared
            .upload_instances(&gpu.queue, &scene.instances, changed)
            .expect("upload");
        self.prepared
            .update_frame(
                &gpu.queue,
                &mut self.target,
                scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            )
            .expect("frame");
        let mut readback = self.target.readback(&gpu.device);
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.prepared
            .encode_with_readback(
                &mut encoder,
                gpu.caps.generation(),
                &self.target,
                None,
                &mut readback,
            )
            .expect("encode");
        gpu.queue.submit([encoder.finish()]);
        gpu.wait();
        let pixels = readback
            .try_collect(gpu.caps.generation())
            .expect("collect")
            .expect("pixels");
        // Repeat the same frame for the public counts API before committing
        // history. Both encodes read the same committed side, so the counts
        // describe the exact list whose pixels we just collected.
        let counted = self
            .prepared
            .submit_counted(
                (&gpu.device, &gpu.queue),
                gpu.caps.generation(),
                &mut self.target,
                None,
            )
            .expect("counts");
        self.prepared.commit_frame();
        gpu.wait();
        (
            pixels,
            counted.take().map(|counts| {
                assert_eq!(counts.mode, self.prepared.camera_culling());
                counts.drawn
            }),
        )
    }
    fn resize(&mut self, gpu: &Gpu, extent: [u32; 2]) {
        self.target = OffscreenTarget::new(&gpu.device, &self.prepared, extent[0], extent[1])
            .expect("resize");
    }
}
fn place(instance: &mut InstanceRecord, half: [f32; 3], center: [f32; 3]) {
    for (axis, scale) in half.into_iter().enumerate() {
        instance.current[axis] = [0.0; 4];
        instance.normal[axis] = [0.0; 4];
        instance.current[axis][axis] = scale;
        instance.normal[axis][axis] = scale.recip();
    }
    instance.current[3] = [center[0], center[1], center[2], 1.0];
    instance.previous = instance.current;
}
fn plane() -> SyntheticMesh {
    let vertex = |position| SyntheticVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        uv: [0.0; 2],
        tangent: [1.0, 0.0, 0.0, 1.0],
    };
    // Faces away from the camera: its back must never supply occluder depth.
    SyntheticMesh {
        vertices: vec![
            vertex([-1.0, -1.0, 0.0]),
            vertex([1.0, -1.0, 0.0]),
            vertex([1.0, 1.0, 0.0]),
            vertex([-1.0, 1.0, 0.0]),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}
fn field() -> SyntheticScene {
    let mut scene = SyntheticScene::grid(900).expect("grid");
    scene.view = SyntheticView::perspective(
        [0.0, 0.0, -2.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        0.9,
        16.0 / 9.0,
        0.05,
        100.0,
    )
    .expect("view");
    for (slot, instance) in scene.instances.iter_mut().enumerate() {
        let x = (slot % 30) as f32 * 0.07 - 0.5;
        let y = (slot / 30) as f32 * 0.035 - 0.5;
        place(
            instance,
            [if slot % 5 == 0 { -0.02 } else { 0.02 }, 0.02, 0.02],
            [x, y, 1.0],
        );
    }
    place(
        &mut scene.instances[899],
        [0.9, 0.9, 0.05],
        [-0.45, 0.0, 0.0],
    );
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        meshes: vec![plane()],
        slots: (0..900)
            .map(|slot| {
                if slot == 898 {
                    SyntheticMeshChoice::Mesh(0)
                } else {
                    SyntheticMeshChoice::Cube
                }
            })
            .collect(),
    }));
    // A large open backface in front of the uncovered side.
    place(&mut scene.instances[898], [0.5, 0.8, 1.0], [0.7, 0.0, 0.0]);
    scene
}
fn equal(a: &[u8], b: &[u8], label: &str) {
    let changed = a
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(a.len(), b.len());
    assert_eq!(changed, 0, "{label}: pixels changed");
}
#[test]
fn churn_reveal_resize_and_device_replacement_keep_bucket_direct() {
    pollster::block_on(async {
        let mut scene = field();
        for generation in [61, 62] {
            let gpu = Gpu::new(generation).await;
            let mut direct = gpu.prepare(&scene, CullMode::Direct, [320, 180]).await;
            let mut frustum = gpu.prepare(&scene, CullMode::Frustum, [320, 180]).await;
            let mut occlusion = gpu.prepare(&scene, CullMode::Occlusion, [320, 180]).await;
            for frame in 0..9 {
                let changed: Vec<u32> = if (2..=5).contains(&frame) {
                    (0..898).collect()
                } else if frame == 6 {
                    vec![0]
                } else {
                    vec![]
                };
                for &slot in &changed {
                    let instance = &mut scene.instances[slot as usize];
                    match frame {
                        2 | 3 => {
                            instance.current[0][0] = -instance.current[0][0];
                            instance.normal[0][0] = -instance.normal[0][0];
                        }
                        4 => instance.ids[3] = u32::from(slot % 3 == 0),
                        5 => instance.ids[3] = 1,
                        6 => place(instance, [0.1; 3], [0.0, 0.0, -0.5]),
                        _ => {}
                    }
                    instance.previous = instance.current;
                }
                if frame == 7 {
                    for sequence in [&mut direct, &mut frustum, &mut occlusion] {
                        sequence.resize(&gpu, [481, 271]);
                    }
                }
                let (a, _) = direct.frame(&gpu, &scene, &changed);
                let (b, f) = frustum.frame(&gpu, &scene, &changed);
                let (c, o) = occlusion.frame(&gpu, &scene, &changed);
                equal(&a, &b, "Frustum");
                equal(&a, &c, "Occlusion");
                if frame > 0 {
                    assert!(
                        o.expect("Occlusion counts") < f.expect("Frustum counts"),
                        "hidden slots, generation {generation} frame {frame}"
                    );
                }
                eprintln!(
                    "generation {generation} frame {frame}: Frustum {f:?}, Occlusion {o:?}, equal"
                );
            }
        }
    });
}

/// A mesh with a planar front exactly at its local min-z bound. Unlike a
/// rotated cube, this has no empty AABB wedge in front of the face.
fn bounded_face(z: f32) -> SyntheticMesh {
    let mut mesh = plane();
    for vertex in &mut mesh.vertices {
        vertex.position[2] = z;
        vertex.normal = [0.0, 0.0, -1.0];
    }
    mesh.indices = vec![0, 2, 1, 0, 3, 2];
    // A rear point fixes a nonzero z extent without contributing triangles.
    let mut rear = mesh.vertices[0];
    rear.position[2] = 1.0;
    mesh.vertices.push(rear);
    mesh.indices.extend([4, 4, 4]);
    mesh
}
#[test]
fn mesh_face_on_its_bounds_keeps_flush_ties_at_rounded_far_poses() {
    pollster::block_on(async {
        let gpu = Gpu::new(63).await;
        for origin in [[-400.37, 1.13, 400.71], [4000.37, 1.13, -4000.71]] {
            for yaw in [0.0_f32, 0.3] {
                let (sin, cos) = yaw.sin_cos();
                let mut scene = SyntheticScene::grid(2).expect("grid");
                // Roll the mesh in its plane: its front still touches the
                // world AABB, unlike a yawed cube with an empty front wedge.
                let mut mesh = bounded_face(-0.731);
                for vertex in &mut mesh.vertices {
                    let [x, y, z] = vertex.position;
                    vertex.position = [cos * x - sin * y, sin * x + cos * y, z];
                }
                scene.meshes = Some(Arc::new(SyntheticMeshes {
                    meshes: vec![mesh, bounded_face(0.0)],
                    slots: vec![SyntheticMeshChoice::Mesh(0), SyntheticMeshChoice::Mesh(1)],
                }));
                scene.view = SyntheticView::perspective(
                    [origin[0], origin[1], origin[2] - 1.5],
                    origin,
                    [0.0, 1.0, 0.0],
                    0.9,
                    16.0 / 9.0,
                    0.1,
                    30000.0,
                )
                .expect("view");
                place(
                    &mut scene.instances[0],
                    [0.25, 0.25, 0.5],
                    [origin[0], origin[1], origin[2]],
                );
                place(
                    &mut scene.instances[1],
                    [1.5, 1.5, 1.0],
                    [origin[0], origin[1], origin[2] - 0.731_f32 * 0.5],
                );
                let mut initial = scene.clone();
                initial.instances[0].ids[3] = 0;
                let mut direct = gpu.prepare(&initial, CullMode::Direct, [320, 180]).await;
                let mut occlusion = gpu.prepare(&initial, CullMode::Occlusion, [320, 180]).await;
                let (wall, _) = direct.frame(&gpu, &initial, &[]);
                occlusion.frame(&gpu, &initial, &[]);
                occlusion.frame(&gpu, &initial, &[]);
                let (a, _) = direct.frame(&gpu, &scene, &[0]);
                let (b, _) = occlusion.frame(&gpu, &scene, &[0]);
                let won = a
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(wall.as_chunks::<4>().0.iter())
                    .filter(|(a, b)| a != b)
                    .count();
                eprintln!("mesh tie origin {origin:?} yaw {yaw}: {won} winning pixels");
                assert!(won > 0, "mesh wins tied pixels");
                equal(&a, &b, "mesh flush tie");
            }
        }
    });
}

#[path = "../src/visibility/occlusion_reference.rs"]
mod reference;

/// Search real projected bounds/vertices for a vertex beyond the unwidened
/// texel rectangle's coverage margin. This does not prove the widening
/// unnecessary; an absence is the explicitly permitted M1 search outcome.
#[test]
fn reference_search_for_a_footprint_widening_witness() {
    use glam::{Mat3, Mat4, Vec3};
    let mut poses = 0_u32;
    let mut widened = 0_u32;
    let mut outside = 0_u32;
    for origin in [
        Vec3::ZERO,
        Vec3::new(-400.37, 1.13, 400.71),
        Vec3::new(4000.37, 1.13, -4000.71),
    ] {
        for yaw_index in 0..32 {
            let yaw = yaw_index as f32 * 0.196_349_55;
            let basis = Mat3::from_rotation_y(yaw);
            for distance in [0.2, 0.5, 1.5, 10.0, 100.0] {
                for size in [0.001, 0.01, 0.1, 1.0] {
                    for offset in [-0.499, -0.25, 0.0, 0.25, 0.499] {
                        let eye = origin + Vec3::new(0.0, 0.0, -distance);
                        let view = SyntheticView::perspective(
                            eye.to_array(),
                            origin.to_array(),
                            [0.0, 1.0, 0.0],
                            0.9,
                            16.0 / 9.0,
                            0.05,
                            30000.0,
                        )
                        .expect("search view");
                        let vp = Mat4::from_cols_array_2d(&view.view_proj_columns());
                        let center = origin + Vec3::new(offset * distance, 0.0, 0.0);
                        let extent =
                            (basis.x_axis.abs() + basis.y_axis.abs() + basis.z_axis.abs()) * size;
                        let mut low = [u32::MAX; 2];
                        let mut high = [0_u32; 2];
                        let mut vertices = Vec::new();
                        let mut safe = true;
                        let mut has_widening = false;
                        let rows = vp.transpose();
                        // Absolute row dot (world magnitude,1), as in WGSL.
                        let world = center.abs() * 2.0 + Vec3::splat(size * 3.0);
                        let magnitude = [
                            rows.x_axis.abs().dot(world.extend(1.0)),
                            rows.y_axis.abs().dot(world.extend(1.0)),
                            rows.z_axis.abs().dot(world.extend(1.0)),
                            rows.w_axis.abs().dot(world.extend(1.0)),
                        ];
                        for corner in 0..8 {
                            let sign = Vec3::new(
                                if corner & 1 == 0 { -1.0 } else { 1.0 },
                                if corner & 2 == 0 { -1.0 } else { 1.0 },
                                if corner & 4 == 0 { -1.0 } else { 1.0 },
                            );
                            let clip = vp * (center + sign * extent).extend(1.0);
                            let vertex = vp * (basis * (sign * size) + center).extend(1.0);
                            if clip.w <= 0.05 || clip.z >= clip.w || vertex.w <= 0.05 {
                                safe = false;
                                break;
                            }
                            let uv = [0.5 + 0.5 * clip.x / clip.w, 0.5 - 0.5 * clip.y / clip.w];
                            vertices.push([
                                0.5 + 0.5 * vertex.x / vertex.w,
                                0.5 - 0.5 * vertex.y / vertex.w,
                            ]);
                            for axis in 0..2 {
                                let n = [256.0, 128.0][axis];
                                #[expect(
                                    clippy::cast_possible_truncation,
                                    clippy::cast_sign_loss,
                                    reason = "floored texel clamped to the nonnegative small texture extent"
                                )]
                                let texel = (uv[axis] * n).floor().clamp(0.0, n - 1.0) as u32;
                                low[axis] = low[axis].min(texel);
                                high[axis] = high[axis].max(texel);
                                has_widening |= reference::footprint_widening(
                                    clip[axis] / clip.w,
                                    clip.w - 2.0 * reference::CLIP_GAMMA * magnitude[3],
                                    magnitude[axis],
                                    magnitude[3],
                                    n,
                                ) > 0.0;
                            }
                        }
                        if !safe {
                            continue;
                        }
                        poses += 1;
                        widened += u32::from(has_widening);
                        outside += u32::from(vertices.iter().any(|uv| {
                            (0..2).any(|axis| {
                                let n = [256.0, 128.0][axis];
                                let coordinate = uv[axis] * n;
                                // A vertex must reach a sample centre beyond the
                                // texel's half-pixel coverage margin to matter.
                                coordinate >= 0.0
                                    && coordinate < n
                                    && (coordinate < low[axis] as f32 - 0.25
                                        || coordinate > high[axis] as f32 + 1.25)
                            })
                        }));
                    }
                }
            }
        }
    }
    eprintln!(
        "footprint reference search: {poses} valid poses; {widened} request widening; {outside} vertex coverage candidates beyond the unwidened rectangle"
    );
    assert!(poses > 5000 && widened > 0);
    assert_eq!(
        outside, 0,
        "a candidate needs a GPU no-widening mutant before claiming this item"
    );
}
