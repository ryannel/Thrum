//! Exercise the actual graph binding and retained shader on Metal.
use super::*;
use crate::synthetic::{CaptureSource, OffscreenTarget, PreparedSynthetic, SyntheticScene};
use crate::timing::TimingCategory;

fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
) -> Vec<u8> {
    let mut readback = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_with_readback(&mut encoder, 1, target, None, &mut readback)
        .unwrap();
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    readback.try_collect(1).unwrap().unwrap()
}

#[test]
fn identity_pre_tone_rebind_preserves_exact_linear_and_sdr_bytes_and_rejects_foreign_reads() {
    pollster::block_on(async {
        let Some(crate::test_device::Opened { device, queue, .. }) = crate::test_device::open(
            "identity_pre_tone_rebind_preserves_exact_linear_and_sdr_bytes_and_rejects_foreign_reads",
            None,
            crate::capabilities::DeviceProfile::Portable,
            &crate::synthetic::tests::support::NO_FEATURES,
        ) else {
            return;
        };
        let scene = SyntheticScene::grid(4).unwrap();
        let prepared = PreparedSynthetic::prepare(&device, &queue, 1, &scene)
            .await
            .unwrap();
        for source in [CaptureSource::SceneLinear, CaptureSource::Sdr] {
            let mut target =
                OffscreenTarget::with_capture_source(&device, &prepared, 65, 37, source).unwrap();
            let before = capture(&device, &queue, &prepared, &target);
            let binding = prepared
                .display
                .prepare_input(
                    &device,
                    target.ids.pre_tone.clone(),
                    PreToneViews {
                        colour: &target.scene_view,
                        motion: &target.motion_view,
                        exposure: &prepared.exposure.samples,
                    },
                )
                .unwrap();
            target.display = binding;
            assert_eq!(before, capture(&device, &queue, &prepared, &target));
            // A second target has the same grid/generation, but different graph
            // resource identities. It cannot be rebound into this compiled pass.
            let foreign =
                OffscreenTarget::with_capture_source(&device, &prepared, 65, 37, source).unwrap();
            target.display = prepared
                .display
                .prepare_input(
                    &device,
                    foreign.ids.pre_tone.clone(),
                    PreToneViews {
                        colour: &foreign.scene_view,
                        motion: &foreign.motion_view,
                        exposure: &prepared.exposure.samples,
                    },
                )
                .unwrap();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            assert!(matches!(
                prepared.encode(&mut encoder, 1, &target, None),
                Err(RenderError::GraphPassMismatch)
            ));
        }
    });
}

#[test]
fn prepared_reconstruction_texture_uses_declared_copy_to_tone_hazard_and_same_sdr_pixels() {
    use crate::graph::{
        Access, AccessKind, FrameGraph, GraphTextures, Pass, PassScope, ResourceDescriptor,
        ResourceScope,
    };
    use std::collections::BTreeSet;
    pollster::block_on(async {
        let Some(crate::test_device::Opened { device, queue, .. }) = crate::test_device::open(
            "prepared_reconstruction_texture_uses_declared_copy_to_tone_hazard_and_same_sdr_pixels",
            None,
            crate::capabilities::DeviceProfile::Portable,
            &crate::synthetic::tests::support::NO_FEATURES,
        ) else {
            return;
        };
        let prepared =
            PreparedSynthetic::prepare(&device, &queue, 1, &SyntheticScene::grid(4).unwrap())
                .await
                .unwrap();
        let linear = OffscreenTarget::with_capture_source(
            &device,
            &prepared,
            65,
            37,
            CaptureSource::SceneLinear,
        )
        .unwrap();
        capture(&device, &queue, &prepared, &linear);
        let sdr = OffscreenTarget::new(&device, &prepared, 65, 37).unwrap();
        let expected = capture(&device, &queue, &prepared, &sdr);
        let stage = prepared.display;
        let mut graph = FrameGraph::default();
        let descriptor = |format, usage| ResourceDescriptor::Texture {
            dimension: crate::graph::TextureKind::D2,
            width: 65,
            height: 37,
            layers: 1,
            mips: 1,
            samples: 1,
            format,
            usage,
        };
        let source = graph
            .resource(
                descriptor(
                    super::super::super::SCENE_FORMAT,
                    wgpu::TextureUsages::COPY_SRC,
                ),
                ResourceScope::Imported { initialized: true },
            )
            .unwrap();
        let colour = graph
            .resource(
                descriptor(
                    super::super::super::SCENE_FORMAT,
                    wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                ),
                ResourceScope::Imported { initialized: false },
            )
            .unwrap();
        let motion = graph
            .resource(
                descriptor(
                    super::super::super::MOTION_FORMAT,
                    wgpu::TextureUsages::TEXTURE_BINDING,
                ),
                ResourceScope::Imported { initialized: true },
            )
            .unwrap();
        let exposure = graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: exposure::SAMPLE_BYTES,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                ResourceScope::Imported { initialized: true },
            )
            .unwrap();
        let producer = graph.pass(Pass {
            name: "identity reconstruction copy".into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: vec![
                Access::new(source, 0..1, AccessKind::CopyRead),
                Access::new(colour.clone(), 0..1, AccessKind::CopyWrite),
            ],
            external_side_effect: false,
            timing: TimingCategory::Effects,
        });
        let input = PreToneInput {
            colour: colour.clone(),
            motion,
            motion_extent: [65, 37],
            exposure,
            extent: [65, 37],
        };
        let (output, consumer) = stage.append_graph(&mut graph, &input, None, true).unwrap();
        let plan = graph.compile(1, &BTreeSet::from([stage.key])).unwrap();
        assert!(plan.hazards.iter().any(|h| h.resource == colour));
        assert!(plan.dependencies.contains(&(producer, consumer)));
        let textures = GraphTextures::new(&device, &plan, crate::capabilities::Recipe::Portable);
        let output_texture = textures.texture(&output).unwrap();
        let replacement = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("prepared reconstruction output"),
            size: wgpu::Extent3d {
                width: 65,
                height: 37,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: super::super::super::SCENE_FORMAT,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let colour_view = replacement.create_view(&wgpu::TextureViewDescriptor::default());
        let binding = stage
            .prepare_input(
                &device,
                input,
                PreToneViews {
                    colour: &colour_view,
                    motion: &linear.motion_view,
                    exposure: &prepared.exposure.samples,
                },
            )
            .unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_texture(
            linear.scene_view.texture().as_image_copy(),
            replacement.as_image_copy(),
            replacement.size(),
        );
        let view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (_, pass) = plan
            .pass_bindings()
            .find(|(id, _)| *id == consumer)
            .unwrap();
        stage
            .encode(
                &mut encoder,
                1,
                &binding,
                pass,
                ToneOutput {
                    resource: &output,
                    view: &view,
                    exposure_side: prepared.history.write_side(),
                },
                None,
            )
            .unwrap();
        let readback = thrum_test_support::readback::Readback::texture(
            &device,
            &mut encoder,
            output_texture,
            thrum_test_support::readback::Mip::level(0),
            ..,
        );
        let submission = queue.submit([encoder.finish()]);
        let actual = readback.wait(&device, submission);
        assert_eq!(actual, expected);
    });
}
