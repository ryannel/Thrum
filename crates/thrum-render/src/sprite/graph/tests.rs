use std::time::Duration;

use super::{CaptureSource, EffectTime, PreparedSprites, SpriteTarget};
use crate::sprite::{EmbeddedSheet, SpriteFrame, SpriteRect, SpriteRow, SpriteScene, SpriteView};

#[test]
fn two_sprite_passes_record_independent_targets_in_one_submission() {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(descriptor);
    let Some(gpu) = thrum_test_support::gpu_on("independent sprite passes", &instance) else {
        return;
    };
    let crate::capabilities::OpenedDevice {
        device,
        queue,
        capabilities,
        ..
    } = pollster::block_on(
        crate::capabilities::DeviceRecipe::default().open_admitted(&instance, &gpu.adapter),
    )
    .expect("open the admitted Metal sprite device");
    let colors = [[240, 60, 72, 255], [46, 208, 141, 255]];
    let scene = SpriteScene::new(
        EmbeddedSheet {
            extent: [2, 1],
            rgba: colors.concat().into(),
            frames: [0, 1]
                .map(|x| SpriteRect {
                    origin: [x, 0],
                    size: [1; 2],
                })
                .into(),
            texels_per_unit: 1.0,
        },
        1,
    );
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut reads = Vec::new();
    let time = EffectTime {
        epoch: 1,
        timeline: 0,
        tick: 1,
        tick_duration: Duration::from_millis(16),
        alpha: 1.0,
        display_phase: 1.0,
        capture: true,
    };
    for (index, extent) in [[16, 12], [12, 16]].into_iter().enumerate() {
        let mut prepared = pollster::block_on(PreparedSprites::prepare(
            &capabilities,
            &queue,
            scene.clone(),
        ))
        .expect("prepared independent sprite pass");
        let mut target = SpriteTarget::prepare(&prepared, extent, CaptureSource::Sdr, None)
            .expect("independent sprite destination");
        let centre = [f64::from(u32::try_from(index).unwrap()) * 4.0, 0.0];
        let frame = SpriteFrame {
            view: SpriteView {
                centre,
                pixels_per_unit: 2.0,
            },
            rows: &[SpriteRow::new(u32::try_from(index).unwrap(), centre)],
        };
        let mut upload = prepared
            .upload(&queue, &mut target, frame, 1.0, false)
            .expect("own view and rows");
        let mut pixels = target.readback(&device);
        prepared
            .record(
                &mut encoder,
                &target,
                None,
                Some(&mut pixels),
                None,
                Some(&mut upload.stats),
                time,
                None,
            )
            .expect("record sprite invocation and its readback");
        assert_eq!(upload.stats.draws, 1);
        reads.push((extent, pixels));
    }
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("both sprite passes retire");
    for (([width, height], mut pixels), color) in reads.into_iter().zip(colors) {
        let rgba = pixels
            .try_collect(capabilities.generation())
            .expect("valid pixels")
            .expect("completed pixels");
        let centre = ((height / 2) * width + width / 2) as usize * 4;
        assert_eq!(&rgba[centre..centre + 4], &color);
        assert_eq!(&rgba[..4], &[0, 0, 0, 255]);
    }
}
