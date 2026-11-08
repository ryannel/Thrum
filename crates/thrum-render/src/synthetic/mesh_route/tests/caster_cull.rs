//! Per-pass Sun caster culling against its oracle, unculled casters. Direct
//! draws every admitted caster into every Sun pass; Frustum on the bucket
//! route draws each pass's own list. Both must write the same depth into
//! each map, bit for bit, and shade the same pixels, as the camera moves
//! (receivers handing off between the near and last cascades), through a
//! resize, with mirrored, thin, off-camera and upstream-clipped casters.

use std::sync::Arc;

use super::{Gpu, differing, gpu, place};
use crate::abi::InstanceRecord;
use crate::synthetic::{
    FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticMeshChoice,
    SyntheticMeshes, SyntheticScene, SyntheticView, shadow,
};
use crate::visibility::CullMode;

const CUBE: SyntheticMeshChoice = SyntheticMeshChoice::Cube;
const PYRAMID: SyntheticMeshChoice = SyntheticMeshChoice::Mesh(0);
const TEXELS: usize = (shadow::SIZE * shadow::SIZE) as usize;

/// One frame's evidence: pixels, each map's depth bits, and each Sun pass's
/// culled caster count (Frustum only).
struct Frame {
    pixels: Vec<u8>,
    depth: Vec<u32>,
    casters: Option<Vec<u32>>,
}

const DEPTH_WGSL: &str = "
@group(0) @binding(0) var maps: texture_depth_2d_array;
@group(0) @binding(1) var<storage, read_write> out: array<f32>;
@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(maps);
    if id.x >= size.x || id.y >= size.y { return; }
    out[(id.z * size.y + id.y) * size.x + id.x] = textureLoad(maps, id.xy, id.z, 0);
}
";

impl Gpu {
    fn prepare_field(
        &self,
        scene: &SyntheticScene,
        slots: Vec<SyntheticMeshChoice>,
    ) -> PreparedSynthetic {
        let mut scene = scene.clone();
        scene.meshes = Some(Arc::new(SyntheticMeshes {
            meshes: vec![super::casters::pyramid()],
            slots,
        }));
        pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
            &self.capabilities,
            &self.queue,
            &scene,
            FrameLighting::Diagnostic(light()),
            1.0,
        ))
        .expect("prepare")
    }

    fn wait(&self) {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .expect("poll");
    }

    /// Every map's depth, read through a texel load (the maps are not copy
    /// sources).
    fn depth_bits(&self, target: &OffscreenTarget) -> Vec<u32> {
        let maps = &target.shadow.as_ref().expect("shadowed target").array_view;
        let bytes = (TEXELS * shadow::MAPS * 4) as u64;
        let out = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("caster cull test depth"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("caster cull test depth staging"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("caster cull test depth"),
                source: wgpu::ShaderSource::Wgsl(DEPTH_WGSL.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("caster cull test depth"),
                layout: None,
                module: &shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("caster cull test depth"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(maps),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(shadow::SIZE / 16, shadow::SIZE / 16, shadow::LAYERS);
        }
        encoder.copy_buffer_to_buffer(&out, 0, &staging, 0, bytes);
        self.queue.submit([encoder.finish()]);
        staging.slice(..).map_async(wgpu::MapMode::Read, |result| {
            result.expect("map");
        });
        self.wait();
        let mapped = staging.slice(..).get_mapped_range().expect("mapped");
        mapped
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| u32::from_le_bytes(*bytes))
            .collect()
    }

    /// One committed frame of `scene` into `target`.
    fn frame(
        &self,
        prepared: &mut PreparedSynthetic,
        target: &mut OffscreenTarget,
        scene: &SyntheticScene,
    ) -> Frame {
        prepared
            .update_frame(
                &self.queue,
                target,
                scene,
                FrameLighting::Diagnostic(light()),
                1.0,
            )
            .expect("update");
        let mut readback = target.readback(&self.device);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode_with_readback(&mut encoder, 41, target, None, &mut readback)
            .expect("encode");
        self.queue.submit([encoder.finish()]);
        prepared.commit_frame();
        self.wait();
        let pixels = readback
            .try_collect(41)
            .expect("collect")
            .expect("submitted");
        let casters = prepared
            .caster_culling
            .as_ref()
            .map(|(culling, _)| culling.lists().map(|(_, args)| self.drawn(args)).collect());
        Frame {
            pixels,
            depth: self.depth_bits(target),
            casters,
        }
    }
}

fn light() -> SyntheticLight {
    SyntheticLight {
        direction_to_light: [0.5, 0.75, 0.3],
        irradiance: [4.0; 3],
    }
}

/// An orthogonal basis scaled by `half`, with its inverse-transpose normals.
fn orient(instance: &mut InstanceRecord, axes: [glam::Vec3; 3], half: [f32; 3], center: [f32; 3]) {
    for (axis, (direction, extent)) in axes.into_iter().zip(half).enumerate() {
        let direction = direction.normalize();
        for row in 0..3 {
            instance.current[axis][row] = direction[row] * extent;
            instance.normal[axis][row] = direction[row] / extent;
        }
    }
    instance.current[3] = [center[0], center[1], center[2], 1.0];
    instance.previous = instance.current;
}

fn view(eye: [f32; 3], target: [f32; 3], aspect: f32) -> SyntheticView {
    SyntheticView::perspective(eye, target, [0.0, 1.0, 0.0], 1.0, aspect, 0.1, 80.0).expect("view")
}

/// A ground plate, then a 15 x 15 field of cubes and pyramids at 6 m
/// spacing (some mirrored, some thin), then one long bar along the Sun ray,
/// upstream of the camera's start.
fn field() -> (SyntheticScene, Vec<SyntheticMeshChoice>) {
    const SIDE: usize = 15;
    let count = 2 + SIDE * SIDE;
    let mut scene = SyntheticScene::grid(u32::try_from(count).expect("small")).expect("field");
    scene.sun_shadows = true;
    scene.view = poses()[0].1;
    place(
        &mut scene.instances[0],
        [60.0, 0.05, 60.0],
        [0.0, -0.05, 0.0],
        0.0,
    );
    let mut slots = vec![CUBE];
    for index in 0..SIDE * SIDE {
        let (column, row) = (index % SIDE, index / SIDE);
        let x = (column as f32 - 7.0) * 6.0 + (row % 3) as f32 * 0.7;
        let z = (row as f32 - 7.0) * 6.0 + (column % 2) as f32 * 1.1;
        let yaw = index as f32 * 0.37;
        let pyramid = index % 3 != 0;
        // Pyramid bounds reach y = +-3, the cube's +-1.
        let lift = if pyramid { 0.3 * 3.0 } else { 0.5 };
        let mut half = if pyramid {
            [0.4, 0.3, 0.4]
        } else {
            [0.5, 0.5, 0.4]
        };
        if index % 7 == 0 {
            half[0] = -half[0]; // mirrored
        }
        if index % 11 == 5 {
            half[2] = 0.002; // a thin sheet
        }
        place(&mut scene.instances[1 + index], half, [x, lift, z], yaw);
        slots.push(if pyramid { PYRAMID } else { CUBE });
    }
    // The bar runs along the Sun ray from 20 m to 140 m upstream of the
    // start pose's ground. The near map clips casters more than eight
    // penumbra texels upstream (about 70 m here), so the bar straddles that
    // clip: only its depth extent keeps it in the near list.
    let sun = glam::Vec3::from(light().direction_to_light).normalize();
    let side = sun.cross(glam::Vec3::Y).normalize();
    let up = side.cross(sun);
    let center = glam::Vec3::new(0.0, 0.0, 6.0) + sun * 80.0;
    orient(
        &mut scene.instances[count - 1],
        [sun, side, up],
        [60.0, 0.3, 0.3],
        center.to_array(),
    );
    slots.push(CUBE);
    (scene, slots)
}

/// Poses: forward along the field, sideways, a low look toward the
/// horizon (receivers across both cascades), then the same poses again at a
/// new extent and aspect.
fn poses() -> Vec<([u32; 2], SyntheticView)> {
    let mut poses = Vec::new();
    for (extent, aspect) in [([320, 180], 16.0 / 9.0), ([200, 240], 5.0 / 6.0)] {
        for step in 0..4 {
            let z = 10.0 - step as f32 * 4.0;
            poses.push((
                extent,
                view([step as f32 * 1.5, 2.0, z], [0.0, 0.5, z - 12.0], aspect),
            ));
        }
        poses.push((extent, view([-8.0, 1.0, 20.0], [10.0, 0.5, -40.0], aspect)));
    }
    poses
}

/// Each pose of Direct (unculled casters) against Frustum (culled casters):
/// map depth bits and pixels that differ, and Frustum's per-pass counts.
fn compare(
    gpu: &Gpu,
    scene: &SyntheticScene,
    slots: &[SyntheticMeshChoice],
) -> Vec<(usize, usize, Vec<u32>)> {
    let mut direct_scene = scene.clone();
    direct_scene.camera_culling = CullMode::Direct;
    let mut frustum_scene = scene.clone();
    frustum_scene.camera_culling = CullMode::Frustum;
    let mut direct = gpu.prepare_field(&direct_scene, slots.to_vec());
    let mut frustum = gpu.prepare_field(&frustum_scene, slots.to_vec());
    assert!(direct.caster_culling.is_none(), "Direct draws every caster");
    assert!(
        frustum.caster_culling.is_some(),
        "Frustum culls its casters"
    );
    let mut rows = Vec::new();
    let mut targets: Option<([u32; 2], OffscreenTarget, OffscreenTarget)> = None;
    for (extent, view) in poses() {
        if targets.as_ref().is_none_or(|(at, ..)| *at != extent) {
            let new = |prepared: &PreparedSynthetic| {
                OffscreenTarget::new(&gpu.device, prepared, extent[0], extent[1]).expect("target")
            };
            targets = Some((extent, new(&direct), new(&frustum)));
        }
        let (_, direct_target, frustum_target) = targets.as_mut().expect("targets");
        direct_scene.view = view;
        frustum_scene.view = view;
        let oracle = gpu.frame(&mut direct, direct_target, &direct_scene);
        let culled = gpu.frame(&mut frustum, frustum_target, &frustum_scene);
        let depth = oracle
            .depth
            .iter()
            .zip(&culled.depth)
            .filter(|(a, b)| a != b)
            .count();
        rows.push((
            depth,
            differing(&oracle.pixels, &culled.pixels),
            culled.casters.expect("counts"),
        ));
    }
    rows
}

#[test]
fn per_pass_caster_lists_draw_the_unculled_depth_and_image() {
    let gpu = gpu();
    let (scene, slots) = field();
    let total = u32::try_from(scene.instances.len()).expect("small");
    let rows = compare(&gpu, &scene, &slots);
    for (pose, (depth, pixels, casters)) in rows.iter().enumerate() {
        eprintln!(
            "pose {pose}: {depth} depth texels and {pixels} px differ; casters near {}, far {}, heights {} of {total}",
            casters[0], casters[1], casters[2]
        );
    }
    for (pose, (depth, pixels, casters)) in rows.iter().enumerate() {
        assert_eq!(
            (*depth, *pixels),
            (0, 0),
            "pose {pose} matches unculled casters"
        );
        assert!(
            casters[0] < casters[1],
            "pose {pose}: the near map lists fewer"
        );
        assert!(casters[0] > 0, "pose {pose}: the near map casts");
    }
    assert!(
        rows.iter()
            .all(|(_, _, casters)| casters[0] * 3 < total * 2),
        "the near map culls a third of the field or more"
    );
    assert!(
        rows.iter().any(|(_, _, casters)| casters[1] < total),
        "the last cascade culls on some pose"
    );
}
