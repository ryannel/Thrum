//! Bucket-ordered Direct is the oracle at every submitted frame, including
//! stale history, class churn, physical compaction and resource replacement.
mod mixed_field_host;

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
    fn new(test_gpu: &thrum_test_support::Gpu, generation: u64) -> Self {
        let adapter = test_gpu.adapter.clone();
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = test_gpu.portable(Some(wgpu::Features::empty()));
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
        updates: None,
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
    let Some(test_gpu) = thrum_test_support::gpu(
        "churn_reveal_resize_and_device_replacement_keep_bucket_direct",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let mut scene = field();
        for generation in [61, 62] {
            let gpu = Gpu::new(&test_gpu.fresh_adapter(), generation);
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
                        4 => instance.set_generation(u32::from(slot % 3 == 0)),
                        5 => instance.set_generation(1),
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
    let Some(test_gpu) = thrum_test_support::gpu(
        "mesh_face_on_its_bounds_keeps_flush_ties_at_rounded_far_poses",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let gpu = Gpu::new(&test_gpu.fresh_adapter(), 63);
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
                    updates: None,
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
                initial.instances[0].deactivate();
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
