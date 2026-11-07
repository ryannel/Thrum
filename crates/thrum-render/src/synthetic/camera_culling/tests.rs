//! The grazing grid view through the prepared scene: occlusion really hides
//! slots, the prepass replays last frame's visible set, and a still frame
//! settles. Pixel equality with Direct is in `tests/visibility.rs`.

use super::super::{FrameLighting, SyntheticLight, SyntheticScene, SyntheticView};
use super::*;
use crate::capabilities::DeviceCapabilities;
use crate::contribution::SceneContribution;

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

/// A box of `half` extents at `center`, turned by `yaw` about +y.
fn place(instance: &mut crate::abi::InstanceRecord, half: [f32; 3], center: [f32; 3], yaw: f32) {
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

/// Membership in the tie witnesses of `tests/visibility.rs`, read from the
/// two lists. In both, one slot is in view for two frames and then a second
/// slot enters onto a tie with it: the equal-depth pair (either slot
/// entering) and the box flush inside a wall (near and far, square and
/// turned). With two slots the counts fix membership: the prepass holds only
/// the slot that stayed, and the final list holds both, so the entering slot
/// is drawn and not hidden by the tie.
#[test]
fn an_entering_slot_on_a_tie_is_drawn_by_the_final_list_alone() {
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
        let lighting = FrameLighting::Diagnostic(SyntheticLight::default());
        let mut cases = Vec::new();
        for entering in [0_usize, 1] {
            let mut base = SyntheticScene::grid(2).expect("grid");
            base.view = SyntheticView::perspective(
                [0.0, 0.0, -1.5],
                [0.0, 0.0, 0.5],
                [0.0, 1.0, 0.0],
                1.0,
                16.0 / 9.0,
                0.01,
                10.0,
            )
            .expect("view");
            let staying = 1 - entering;
            base.instances[staying].current[3] = [0.0, 0.0, 0.5, 1.0];
            base.instances[staying].previous = base.instances[staying].current;
            base.instances[entering].current[3] = [40.0, 0.0, 0.5, 1.0];
            base.instances[entering].previous = base.instances[entering].current;
            let mut tie = base.clone();
            let mut moved = tie.instances[staying].clone();
            moved.ids = tie.instances[entering].ids;
            tie.instances[entering] = moved;
            cases.push((
                format!("equal depth, slot {entering} entering"),
                base,
                tie,
                entering,
            ));
        }
        for origin in [[0.0, 1.0, 0.0], [-400.37, 1.13, 400.71]] {
            for yaw in [0.0_f32, 0.3] {
                let (sin, cos) = yaw.sin_cos();
                let at = |offset: [f32; 3]| -> [f32; 3] {
                    let turned = [
                        cos * offset[0] + sin * offset[2],
                        offset[1],
                        -sin * offset[0] + cos * offset[2],
                    ];
                    std::array::from_fn(|axis| origin[axis] + turned[axis])
                };
                let mut base = SyntheticScene::grid(2).expect("grid");
                base.view = SyntheticView::perspective(
                    at([0.0, 0.0, 0.5]),
                    at([0.0, 0.0, 2.0]),
                    [0.0, 1.0, 0.0],
                    46.0_f32.to_radians(),
                    16.0 / 9.0,
                    0.1,
                    30_000.0,
                )
                .expect("field projection");
                place(
                    &mut base.instances[1],
                    [2.0, 1.5, 0.5],
                    at([0.0, 0.0, 2.0]),
                    yaw,
                );
                let half = [0.25; 3];
                place(&mut base.instances[0], half, at([60.0, 0.0, 1.75]), yaw);
                let mut tie = base.clone();
                place(&mut tie.instances[0], half, at([0.1, -0.05, 1.75]), yaw);
                cases.push((format!("flush at {origin:?}, yaw {yaw}"), base, tie, 0));
            }
        }
        for (name, mut base, mut tie, entering) in cases {
            base.camera_culling = CullMode::Occlusion;
            tie.camera_culling = CullMode::Occlusion;
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &base,
                lighting,
                1.0,
            )
            .await
            .expect("prepare");
            let mut target = OffscreenTarget::new(&device, &prepared, 320, 180).expect("target");
            let mut counts = Vec::new();
            for frame in 0..3 {
                let scene = if frame == 2 {
                    let slot = u32::try_from(entering).expect("slot");
                    prepared
                        .upload_instances(&queue, &tie.instances, &[slot])
                        .expect("upload");
                    &tie
                } else {
                    &base
                };
                prepared
                    .update_frame(&queue, &mut target, scene, lighting, 1.0)
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
                    .expect("occlusion")
                    .target
                    .prepass();
                counts.push((
                    instances(&device, &queue, prepass.args()),
                    instances(&device, &queue, drawn.args()),
                ));
            }
            assert_eq!(
                counts,
                [(0, 1), (1, 1), (1, 2)],
                "{name}: (prepass, final) per frame"
            );
        }
    });
}

/// A visible contribution: a half-transparent triangle in front of the
/// cube, alpha-blended at the end of forward. An opaque draw recorded
/// after it would overwrite the blend, which a no-op fixture cannot show.
#[derive(Default)]
pub(crate) struct Veil {
    pipeline: Option<wgpu::RenderPipeline>,
}

const VEIL: &str = "
    @vertex fn veil_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
        let corners = array(vec2f(-0.8, -0.8), vec2f(0.8, -0.8), vec2f(0.0, 0.8));
        return vec4f(corners[index], 0.99, 1.0);
    }
    struct Targets { @location(0) colour: vec4f, @location(1) motion: vec4f }
    @fragment fn veil_fragment() -> Targets {
        return Targets(vec4f(0.2, 0.6, 1.0, 0.5), vec4f(0.0));
    }";

impl SceneContribution for Veil {
    fn passes(&self) -> crate::contribution::ContributionPasses {
        crate::contribution::ContributionPasses::default()
    }
    fn rebuild(&mut self, surface: &crate::contribution::SceneSurface<'_>) -> Result<(), String> {
        let device = surface.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("veil"),
            source: wgpu::ShaderSource::Wgsl(VEIL.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("veil"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        self.pipeline = Some(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("veil"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("veil_vertex"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: surface.depth_format,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Greater),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("veil_fragment"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: surface.color_formats[0],
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: surface.color_formats[1],
                            blend: None,
                            write_mask: wgpu::ColorWrites::empty(),
                        }),
                    ],
                }),
                multiview_mask: None,
                cache: None,
            }),
        );
        Ok(())
    }
    fn encode_upload(
        &mut self,
        _: &mut wgpu::CommandEncoder,
        _: &crate::contribution::ContributionFrame,
    ) {
    }
    fn computes(&self, _: &crate::contribution::ContributionFrame) -> bool {
        false
    }
    fn encode_compute(
        &mut self,
        _: &mut wgpu::ComputePass<'_>,
        _: &crate::contribution::ContributionFrame,
    ) {
    }
    fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {}
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(pipeline) = &self.pipeline {
            pass.set_pipeline(pipeline);
            pass.draw(0..3, 0..1);
        }
    }
    fn submitted(&mut self, _: bool) {}
}

/// Three frames of `scene` with a final readback each, with the veil drawn at
/// the end of forward when `veiled`. Returns each frame's pixels and final
/// list count (`None` for Direct).
async fn veiled_frames(
    (capabilities, device, queue): (&DeviceCapabilities, &wgpu::Device, &wgpu::Queue),
    scene: &SyntheticScene,
    veiled: bool,
) -> Vec<(Vec<u8>, Option<u32>)> {
    let lighting = FrameLighting::Diagnostic(SyntheticLight::default());
    let mut prepared =
        PreparedSynthetic::prepare_with_capabilities(capabilities, queue, scene, lighting, 1.0)
            .await
            .expect("prepare");
    let mut veil = Veil::default();
    veil.rebuild(&prepared.scene_surface(queue)).expect("veil");
    let mut target = OffscreenTarget::new(device, &prepared, 320, 180).expect("target");
    if scene.camera_culling == CullMode::Occlusion {
        // The forward pass, the only place the contribution draws, runs after
        // every pass that builds or reads the prepass depth.
        let order: Vec<&str> = target
            .graph
            .plan()
            .pass_bindings()
            .map(|(_, pass)| pass.name.as_str())
            .collect();
        let at = |name: &str| order.iter().position(|&pass| pass == name).expect(name);
        let forward = at("synthetic forward opaque");
        for name in [
            visibility::PREPASS_LIST_PASS,
            DEPTH_PREPASS_PASS,
            visibility::PYRAMID_PASS,
            visibility::FINAL_PASS,
        ] {
            assert!(at(name) < forward, "{name} runs before forward: {order:?}");
        }
    }
    let mut frames = Vec::new();
    for _ in 0..3 {
        prepared
            .update_frame(queue, &mut target, scene, lighting, 1.0)
            .expect("update");
        let mut readback = target.readback(device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let contribution: Option<&mut (dyn SceneContribution + 'static)> =
            if veiled { Some(&mut veil) } else { None };
        prepared
            .encode_contributed(
                &mut encoder,
                capabilities.generation(),
                &target,
                None,
                Some(&mut readback),
                contribution,
            )
            .expect("encode");
        queue.submit([encoder.finish()]);
        prepared.commit_frame();
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let pixels = readback
            .try_collect(capabilities.generation())
            .expect("collect")
            .expect("mapped pixels");
        let drawn = target
            .visibility
            .as_ref()
            .map(|list| instances(device, queue, list.args()));
        frames.push((pixels, drawn));
    }
    frames
}

/// The contribution witness with an occluder: slot 1, a small cube straight
/// behind slot 0's, is in view and hidden. With the veil attached, Occlusion
/// drops slot 1 from its final list once settled, and every frame, cold and
/// settled, is byte-equal to Direct's with the veil. The veil covers cube
/// pixels, so a contribution drawn before the final list (whose opaque draw
/// would then cover it) fails here. The contribution cannot reach the prepass
/// or the pyramid: its draw is recorded only in the forward pass, which the
/// graph orders after the final cull, and the depth prepass has no colour
/// target to draw it into.
#[test]
fn a_blended_contribution_stays_on_top_while_occlusion_hides_a_slot() {
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
        let gpu = (&capabilities, &device, &queue);
        let mut scene = SyntheticScene::grid(2).expect("grid");
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, -1.5],
            [0.0, 0.0, 0.5],
            [0.0, 1.0, 0.0],
            1.0,
            16.0 / 9.0,
            0.01,
            10.0,
        )
        .expect("view");
        scene.instances[0].current[3] = [0.0, 0.0, 0.5, 1.0];
        scene.instances[0].previous = scene.instances[0].current;
        place(&mut scene.instances[1], [0.03; 3], [0.0, 0.0, 1.5], 0.0);
        let plain = veiled_frames(gpu, &scene, false).await;
        let mut uncovered = scene.clone();
        uncovered.instances[0].ids[3] = 0;
        let behind = veiled_frames(gpu, &uncovered, false).await;
        let direct = veiled_frames(gpu, &scene, true).await;
        let mut runs = Vec::new();
        for mode in [CullMode::Frustum, CullMode::Occlusion] {
            let mut culled = scene.clone();
            culled.camera_culling = mode;
            runs.push(veiled_frames(gpu, &culled, true).await);
        }
        // The veil lies over the cube: pixels the cube owns that the veil changes.
        let overlap = (0..direct[2].0.len() / 4)
            .filter(|&texel| {
                let at = |image: &[u8]| image[texel * 4..texel * 4 + 4].to_vec();
                at(&direct[2].0) != at(&plain[2].0) && at(&plain[2].0) != at(&behind[2].0)
            })
            .count();
        assert!(overlap > 1_000, "the veil covers the cube: {overlap}");
        let counts: Vec<Vec<Option<u32>>> = runs
            .iter()
            .map(|frames| frames.iter().map(|frame| frame.1).collect())
            .collect();
        assert_eq!(counts[0], [Some(2); 3], "Frustum draws both");
        assert_eq!(
            counts[1],
            [Some(2), Some(1), Some(1)],
            "cold Occlusion draws the frustum set, then hides slot 1"
        );
        for (mode, frames) in ["Frustum", "Occlusion"].into_iter().zip(&runs) {
            for (frame, ((culled, _), (reference, _))) in frames.iter().zip(&direct).enumerate() {
                assert!(
                    culled == reference,
                    "{mode} frame {frame} differs from Direct"
                );
            }
        }
    });
}
