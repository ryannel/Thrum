use super::*;
use crate::{
    runtime::{CaptureLimits, SurfaceOutput},
    synthetic::{CaptureSource, NativeWitness},
};

fn output(device: &wgpu::Device, extent: [u32; 2]) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("native lifecycle surface stand-in"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn drain(host: &mut Host) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while host.pending_frames() != 0 {
        let timeout = deadline.saturating_duration_since(std::time::Instant::now());
        assert!(!timeout.is_zero(), "native frames did not retire");
        host.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(timeout),
            })
            .expect("native completion");
        while host.poll_completed().expect("collect native").is_some() {}
    }
}

fn image(host: &mut Host) -> Vec<u8> {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        host.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("capture completion");
        if let Some(image) = host.try_submitted_image().expect("submitted native image") {
            let bytes = image.rgba.clone();
            host.recycle_submitted_image(image);
            drain(host);
            return bytes;
        }
        assert!(std::time::Instant::now() < deadline, "native tap timed out");
    }
}

async fn sequence(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    selection: NativeWitness,
    source: CaptureSource,
) -> Vec<Vec<u8>> {
    let mut host = host(
        capabilities,
        queue,
        Policy {
            native: selection,
            ..Policy::default()
        },
    )
    .await;
    host.enable_submitted_capture_from(1, CaptureLimits::default(), source)
        .expect("native tap");
    let view = output(capabilities.device(), [64, 64]);
    let surface = SurfaceOutput {
        view: &view,
        extent: [64, 64],
    };
    let mut frames = Vec::new();
    for tick in 0..8 {
        host.world.authority.tick = tick;
        host.world.publication.tick = tick;
        host.source.camera = tick as f32 * 0.05;
        if tick == 1 {
            host.invalidate_history();
        }
        if tick == 3 {
            host.advance(Duration::ZERO)
                .expect("discard advance")
                .encode_scene()
                .expect("native discard recording")
                .skip();
        }
        if tick == 4 {
            let flushes = super::super::super::tokens::staging_flush::attempts();
            host.advance(Duration::ZERO)
                .expect("background advance")
                .encode_scene()
                .expect("native background recording")
                .cancel_background();
            assert_eq!(
                super::super::super::tokens::staging_flush::attempts(),
                flushes,
                "no background queue flush"
            );
        }
        if tick == 5 {
            let submitted = host.native_witness_report().unwrap().submitted;
            host.advance(Duration::ZERO)
                .unwrap()
                .encode_scene()
                .unwrap()
                .submit(None)
                .unwrap();
            assert_eq!(
                host.native_witness_report().unwrap().submitted,
                submitted,
                "acquire refusal must discard native work rather than submit without present"
            );
        }
        host.advance(Duration::ZERO)
            .expect("native advance")
            .encode_scene()
            .expect("native record")
            .submit(Some(&surface))
            .expect("native submit");
        frames.push(image(&mut host));
    }
    let report = host.native_witness_report().expect("native outcomes");
    assert_eq!(report.selection, selection);
    assert_eq!(report.submitted, 8);
    assert_eq!(report.discarded, 3);
    assert_eq!(report.completed, 8);
    assert_eq!(report.encoded, 11);
    assert_eq!(host.submitted_capture_omissions(), 0);
    frames
}

#[test]
fn metal_coordinator_native_history_reset_discard_and_exact_twin() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        for source in [
            CaptureSource::Sdr,
            CaptureSource::SceneLinear,
            CaptureSource::NativeOutput,
            CaptureSource::NativeHistory,
        ] {
            let twin = sequence(&capabilities, &queue, NativeWitness::Wgsl, source).await;
            let metal = sequence(&capabilities, &queue, NativeWitness::Metal, source).await;
            assert_eq!(metal, twin, "exact coordinator {source:?}");
            if source != CaptureSource::Sdr {
                for value in metal.iter().flat_map(|frame| frame.as_chunks::<2>().0) {
                    let value = half::f16::from_le_bytes([value[0], value[1]]);
                    assert!(value.is_finite() && value.to_f32() >= 0.0);
                }
            }
            if source == CaptureSource::NativeOutput || source == CaptureSource::NativeHistory {
                for fault in [
                    NativeWitness::NoCommands,
                    NativeWitness::NoTile,
                    NativeWitness::NoReset,
                    NativeWitness::StaleSource,
                    NativeWitness::StaleHistory,
                ] {
                    assert_ne!(
                        sequence(&capabilities, &queue, fault, source).await,
                        twin,
                        "{fault:?} must fail exact {source:?}"
                    );
                }
            }
        }
        println!(
            "M10 coordinator: exact SDR, scene input, witness output/history; five controls rejected; healthy discard/background reset counters exact"
        );
    });
}

#[test]
fn metal_native_submission_failure_loss_and_extent_replacement_reset_history() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, queue) = gpu(&instance, 7).await;
        let mut host = host(
            &first,
            &queue,
            Policy {
                native: NativeWitness::Metal,
                ..Policy::default()
            },
        )
        .await;
        let view = output(first.device(), [64, 64]);
        let surface = SurfaceOutput {
            view: &view,
            extent: [64, 64],
        };
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&surface))
            .unwrap();
        drain(&mut host);
        let prior = host.prepared.history_identity();
        let mut advanced = host.advance(Duration::ZERO).unwrap();
        let mut report = advanced.report.take().unwrap();
        let owner = advanced.host.take().unwrap();
        let frame = owner.extract_serial(Duration::ZERO, 0, 0).unwrap();
        let prepared = owner
            .prepare_extracted(0, &mut report, frame)
            .unwrap()
            .unwrap();
        let recorded = owner.encode_scene_only(0, prepared).unwrap();
        assert!(
            owner
                .finish_encoded_with(
                    recorded,
                    &mut report,
                    Some(&surface),
                    std::time::Instant::now(),
                    || Err(FrameError::Gpu("forced native submission refusal".into()))
                )
                .is_err()
        );
        assert_eq!(owner.prepared.history_identity(), prior);
        assert_eq!(owner.native_witness_report().unwrap().discarded, 1);
        let flushes = super::super::super::tokens::staging_flush::attempts();
        let inputs = owner.device_prepare_inputs().unwrap();
        let lost_device = owner.device.clone();
        let encoded = owner
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap();
        lost_device.destroy();
        encoded.abandon_lost_device();
        assert_eq!(
            super::super::super::tokens::staging_flush::attempts(),
            flushes
        );
        assert_eq!(owner.native_witness_report().unwrap().discarded, 2);
        let (second, queue2) = gpu(&instance, 8).await;
        let mut candidate = inputs
            .prepare(&second, &queue2, Some(super::surface()))
            .await
            .unwrap();
        candidate.resize_scene_surface([96, 48]).unwrap();
        owner.publish_device(&mut candidate).unwrap();
        assert_eq!(owner.world.source().epoch, 5);
        assert_eq!(owner.native_witness_report().unwrap().submitted, 0);
        owner
            .enable_submitted_capture_from(2, CaptureLimits::default(), CaptureSource::NativeOutput)
            .unwrap();
        let view = output(second.device(), [96, 48]);
        owner
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &view,
                extent: [96, 48],
            }))
            .unwrap();
        let replaced = image(owner);
        assert_eq!(owner.native_witness_report().unwrap().resets, 1);
        let mut twin = super::host(
            &second,
            &queue2,
            Policy {
                native: NativeWitness::Wgsl,
                ..Policy::default()
            },
        )
        .await;
        twin.resize_scene_surface([96, 48]).unwrap();
        twin.enable_submitted_capture_from(
            2,
            CaptureLimits::default(),
            CaptureSource::NativeOutput,
        )
        .unwrap();
        twin.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &view,
                extent: [96, 48],
            }))
            .unwrap();
        assert_eq!(
            replaced,
            image(&mut twin),
            "replacement resets to the exact prepared twin"
        );
        println!(
            "M10 native failure/loss/extent: submission refusal and loss abandon discarded=2, no loss flush; new generation/extent reset before reuse"
        );
    });
}

#[test]
fn metal_native_generation_resources_survive_until_completion_delivery() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, queue) = gpu(&instance, 7).await;
        let mut host = host(
            &first,
            &queue,
            Policy {
                native: NativeWitness::Metal,
                ..Policy::default()
            },
        )
        .await;
        let (second, queue2) = gpu(&instance, 8).await;
        let mut candidate = host
            .device_prepare_inputs()
            .unwrap()
            .prepare(&second, &queue2, Some(surface()))
            .await
            .unwrap();
        let weak = Arc::downgrade(host.slots[0].target.native.as_ref().unwrap());
        let old_device = host.device.clone();
        let view = output(first.device(), [64, 64]);
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &view,
                extent: [64, 64],
            }))
            .unwrap();
        assert_eq!(
            host.native_witness_report().unwrap().completed,
            0,
            "completion deliberately not delivered"
        );
        host.publish_device(&mut candidate).unwrap();
        assert!(
            weak.upgrade().is_some(),
            "native binding/provider retained across replacement while completion is outstanding"
        );
        old_device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .unwrap();
        assert!(
            weak.upgrade().is_none(),
            "old native generation released after completion delivery"
        );
        assert_eq!(
            host.native_witness_report().unwrap().completed,
            0,
            "old completion cannot publish into new provider"
        );
        println!(
            "M10 native generation retention: old provider/source/textures retained until old completion delivery, stale callback cannot mutate replacement"
        );
    });
}

#[test]
fn metal_native_two_slots_select_submitted_history_before_completion() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let view = output(capabilities.device(), [64, 64]);
        let surface = SurfaceOutput {
            view: &view,
            extent: [64, 64],
        };
        let mut pairs = Vec::new();
        for selection in [NativeWitness::Wgsl, NativeWitness::Metal] {
            let mut host = host(
                &capabilities,
                &queue,
                Policy {
                    native: selection,
                    ..Policy::default()
                },
            )
            .await;
            host.enable_submitted_capture_from(
                1,
                CaptureLimits::default(),
                CaptureSource::NativeHistory,
            )
            .unwrap();
            host.advance(Duration::ZERO)
                .unwrap()
                .encode_scene()
                .unwrap()
                .submit(Some(&surface))
                .unwrap();
            let (report, advance) = host.admit_world(Duration::ZERO).unwrap();
            let held = crate::runtime::AdvancedFrame {
                host: Some(&mut host),
                report: Some(report),
                tick_cpu: advance.tick_cpu,
                wall_start: std::time::Instant::now(),
                recovered_publication: advance.recovered_publication,
            }
            .encode_scene()
            .unwrap();
            held.submit(Some(&surface)).unwrap();
            assert_eq!(host.pending_frames(), 2);
            assert_eq!(host.native_witness_report().unwrap().submitted, 2);
            let first = image(&mut host);
            let second = image(&mut host);
            assert_eq!(&first[..8], &[0, 60, 0, 60, 0, 60, 0, 60]);
            assert_eq!(&second[..8], &[0; 8]);
            pairs.push([first, second]);
        }
        assert_eq!(
            pairs[0], pairs[1],
            "two in-flight native histories exactly match the twin"
        );
        println!(
            "M10 native two-slot history: both frames submitted before completion polling; prior submitted side selected exactly"
        );
    });
}
