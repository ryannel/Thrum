//! Real-Metal submitted-image identity, ordinary presenter and bounded leases.

use super::*;
use crate::{present::SurfaceProfile, runtime::SurfaceOutput};
use std::time::Instant;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct AllocationObserver;
#[global_allocator]
static ALLOCATOR: AllocationObserver = AllocationObserver;
#[expect(
    unsafe_code,
    reason = "test-only allocation observer forwards unchanged pointers and layouts to System"
)]
// SAFETY: The observer never owns memory; every operation delegates unchanged
// to System. Counters are thread-local and only active in the scoped test.
unsafe impl GlobalAlloc for AllocationObserver {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNTING.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        // SAFETY: The original layout is passed unchanged to System.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The pointer/layout came from the delegated System allocator.
        unsafe {
            System.dealloc(pointer, layout);
        }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        COUNTING.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        // SAFETY: Original pointer/layout and requested size are forwarded.
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[test]
fn disabled_capture_frame_hook_allocates_nothing_and_has_no_readback_graph_pass() {
    let mut disabled = None;
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|active| active.set(true));
    for _ in 0..10000 {
        assert!(super::super::submitted_capture::reserve(&mut disabled, 1, [65, 37]).is_none());
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    pollster::block_on(async {
        let (host, _, _) = Box::pin(host_and_output(wgpu::TextureFormat::Rgba8Unorm)).await;
        let off = host.prepared.graph(&host.slots[0].target, false).unwrap();
        let on = host.prepared.graph(&host.slots[0].target, true).unwrap();
        assert!(
            !off.pass_bindings()
                .any(|(_, pass)| pass.kind == Some(crate::graph::PassKind::Copy))
        );
        assert!(
            on.pass_bindings()
                .any(|(_, pass)| pass.kind == Some(crate::graph::PassKind::Copy))
        );
        assert_eq!(host.submitted_capture_usage(), None);
    });
}

async fn host_and_output(
    format: wgpu::TextureFormat,
) -> (
    FrameHost<StaticWorld, StaticScene>,
    wgpu::Texture,
    wgpu::Instance,
) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (capabilities, queue) = device::gpu(&instance, 7).await;
    let extent = [65, 37];
    let source = SourceTick {
        epoch: 2,
        tick: 0,
        timeline: 1,
    };
    let host = FrameHost::new(
        &capabilities,
        &queue,
        FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile { format, extent }),
        },
        StaticWorld {
            authority: source,
            publication: source,
        },
        StaticScene::default(),
        SyntheticScene::grid(1).unwrap(),
        1.0,
    )
    .await
    .unwrap();
    let output = host.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("submitted-image reference output"),
        size: wgpu::Extent3d {
            width: extent[0],
            height: extent[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    (host, output, instance)
}

fn submit(host: &mut FrameHost<StaticWorld, StaticScene>, output: &wgpu::Texture) -> FrameSample {
    let view = output.create_view(&wgpu::TextureViewDescriptor::default());
    let report = host
        .drive(
            Duration::ZERO,
            Some(&SurfaceOutput {
                view: &view,
                extent: [65, 37],
            }),
        )
        .unwrap();
    assert!(
        report.submitted,
        "ordinary paused-world surface gate must submit"
    );
    assert_eq!(report.tick, 0);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut completed = None;
    while host.pending_frames() != 0 {
        completed = host.poll_completed().unwrap().or(completed);
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    completed.expect("ordinary submitted frame completed")
}

fn image(host: &mut FrameHost<StaticWorld, StaticScene>) -> super::super::SubmittedImage {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(image) = host.try_submitted_image().unwrap() {
            return image;
        }
        let _ = host.poll_completed().unwrap();
        assert!(Instant::now() < deadline);
    }
}

#[test]
fn submitted_tap_matches_the_actual_odd_width_surface_and_preserves_history() {
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ] {
        pollster::block_on(async {
            let (mut host, output, _instance) = Box::pin(host_and_output(format)).await;
            host.enable_submitted_capture(42).unwrap();
            #[cfg(feature = "creation-counters")]
            {
                let report = host.creation_report();
                assert_eq!(report.violations, 0);
                assert!(
                    report
                        .pipelines
                        .iter()
                        .any(|row| row.label == "capture-copy-timing"
                            && row.phase == crate::runtime::CreationPhase::Transition
                            && row.compute > 0)
                );
            }
            host.set_presentation_lead(Duration::from_millis(7));
            submit(&mut host, &output);
            let image = image(&mut host);
            assert!(
                image
                    .copy_gpu_ns
                    .is_some_and(|ns| ns.is_finite() && ns > 0.0)
            );
            assert_eq!(image.device_generation, 7);
            assert_eq!(image.presenter_generation, 1);
            assert_eq!(image.presentation_lead, Duration::from_millis(7));
            assert_eq!(image.rgba.len(), 65 * 37 * 4);
            assert_eq!(image.session, 42);
            assert_eq!(image.source, host.world.source());
            assert_eq!(image.sample.submission_order, 0);
            assert_eq!(image.sample.tick, 0);
            assert!(image.sample.surface_presented);
            assert_eq!(image.history_epoch, host.submitted_view_history_epoch);
            // Independently copy the exact output consumed by the presenter; no
            // second scene render or endpoint capture participates in comparison.
            let buffer = host.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 512 * 37,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = host
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            encoder.copy_texture_to_buffer(
                output.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(512),
                        rows_per_image: Some(37),
                    },
                },
                wgpu::Extent3d {
                    width: 65,
                    height: 37,
                    depth_or_array_layers: 1,
                },
            );
            let (send, receive) = std::sync::mpsc::channel();
            encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |result| {
                send.send(result).unwrap();
            });
            host.queue.submit([encoder.finish()]);
            host.device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
            receive.recv().unwrap().unwrap();
            let mapped = buffer.slice(..).get_mapped_range().unwrap();
            let mut expected: Vec<_> = mapped
                .as_chunks::<512>()
                .0
                .iter()
                .flat_map(|row| row[..65 * 4].iter().copied())
                .collect();
            if format == wgpu::TextureFormat::Bgra8Unorm {
                for pixel in expected.as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2);
                }
            }
            assert_eq!(
                image.rgba, expected,
                "tap must match the final present texture byte for byte"
            );
            let epoch = host.submitted_view_history_epoch;
            host.recycle_submitted_image(image);
            host.disable_submitted_capture();
            submit(&mut host, &output);
            assert_eq!(host.submitted_view_history_epoch, epoch);
            assert!(host.submitted_capture.is_none());
            assert_eq!(host.submitted_capture_omissions(), 0);
        });
    }
}

#[test]
fn saturated_cpu_and_gpu_leases_do_not_stop_ordinary_submissions() {
    pollster::block_on(async {
        let (mut host, output, _instance) =
            Box::pin(host_and_output(wgpu::TextureFormat::Rgba8Unorm)).await;
        host.enable_submitted_capture(1).unwrap();
        let mut held = Vec::new();
        for _ in 0..8 {
            submit(&mut host, &output);
            held.push(image(&mut host));
        }
        for _ in 0..7 {
            submit(&mut host, &output);
        }
        assert!(host.try_submitted_image().unwrap().is_none());
        assert_eq!(host.submitted_capture_omissions(), 4);
        assert_eq!(host.next_submission_order, 15);
        assert_eq!(host.submitted_capture_usage(), Some((3, 8)));
        for image in held {
            host.recycle_submitted_image(image);
        }
        for _ in 0..3 {
            let image = image(&mut host);
            host.recycle_submitted_image(image);
        }
        host.disable_submitted_capture();
        host.enable_submitted_capture(2).unwrap();
        submit(&mut host, &output);
        let pending = image(&mut host);
        host.disable_submitted_capture();
        host.enable_submitted_capture(2).unwrap();
        submit(&mut host, &output);
        let fresh = image(&mut host);
        host.recycle_submitted_image(pending);
        assert_eq!(host.submitted_capture_usage(), Some((0, 1)));
        host.recycle_submitted_image(fresh);
        assert_eq!(host.submitted_capture_usage(), Some((0, 0)));
        submit(&mut host, &output);
        assert_eq!(image(&mut host).session, 2);
    });
}

#[test]
fn pending_capture_device_loss_is_rejected_without_polling_the_lost_queue() {
    pollster::block_on(async {
        let (mut host, output, instance) =
            Box::pin(host_and_output(wgpu::TextureFormat::Rgba8Unorm)).await;
        host.enable_submitted_capture(1).unwrap();
        let view = output.create_view(&wgpu::TextureViewDescriptor::default());
        assert!(
            host.drive(
                Duration::ZERO,
                Some(&SurfaceOutput {
                    view: &view,
                    extent: [65, 37]
                })
            )
            .unwrap()
            .submitted
        );
        host.device.destroy();
        let inputs = host.device_prepare_inputs().unwrap();
        let (capabilities, queue) = device::gpu(&instance, 8).await;
        let mut replacement = inputs
            .prepare(
                &capabilities,
                &queue,
                Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    extent: [65, 37],
                }),
            )
            .await
            .unwrap();
        host.publish_device(&mut replacement).unwrap();
        assert!(matches!(
            host.try_submitted_image(),
            Err(super::super::FrameError::InvalidState(
                "retired capture generation"
            ))
        ));
        let output = host.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 65,
                height: 37,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        // An invalidated observer never prevents the continuing app submitting
        // on its replacement device, even before the caller ends capture.
        submit(&mut host, &output);
        assert_eq!(host.submitted_capture_omissions(), 1);
        host.disable_submitted_capture();
        host.enable_submitted_capture(2).unwrap();
        submit(&mut host, &output);
        let fresh = image(&mut host);
        assert_eq!(fresh.session, 2);
        assert_eq!(fresh.device_generation, 8);
        assert_eq!(fresh.presenter_generation, 2);
    });
}

#[test]
fn abandoned_unsubmitted_capture_releases_its_slot_without_lost_queue_work() {
    pollster::block_on(async {
        let (mut host, output, _) =
            Box::pin(host_and_output(wgpu::TextureFormat::Rgba8Unorm)).await;
        host.enable_submitted_capture(1).unwrap();
        let before = super::super::tokens::staging_flush::attempts();
        let encoded = host
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap();
        encoded.abandon_lost_device();
        assert_eq!(super::super::tokens::staging_flush::attempts(), before);
        assert_eq!(host.submitted_capture_usage(), Some((0, 0)));
        assert_eq!(host.submitted_capture_omissions(), 1);
        submit(&mut host, &output);
        assert_eq!(image(&mut host).sample.submission_order, 0);
    });
}

#[test]
fn pending_scene_grid_pixels_keep_their_identity_across_surface_viewport_resize() {
    pollster::block_on(async {
        let (mut host, output, _) =
            Box::pin(host_and_output(wgpu::TextureFormat::Rgba8Unorm)).await;
        assert!(
            host.enable_submitted_capture_with_limits(
                1,
                super::super::CaptureLimits {
                    staging_slots: 3,
                    cpu_buffers: 8,
                    byte_allowance: 1,
                }
            )
            .is_err()
        );
        assert_eq!(host.submitted_capture_usage(), None);
        host.enable_submitted_capture(1).unwrap();
        let view = output.create_view(&wgpu::TextureViewDescriptor::default());
        assert!(
            host.drive(
                Duration::ZERO,
                Some(&SurfaceOutput {
                    view: &view,
                    extent: [65, 37]
                })
            )
            .unwrap()
            .submitted
        );
        host.resize_surface([83, 41]).unwrap();
        let old = image(&mut host);
        assert_eq!(old.sample.submission_order, 0);
        assert_eq!(old.presenter_generation, 1);
        assert_eq!(
            old.rgba.len(),
            65 * 37 * 4,
            "scene-grid capture is not relabelled as the resized platform viewport"
        );
        host.recycle_submitted_image(old);
        host.resize_surface([65, 37]).unwrap();
        while host.pending_frames() > 0 {
            let _ = host.poll_completed().unwrap();
        }
        submit(&mut host, &output);
        let fresh = image(&mut host);
        assert_eq!(fresh.sample.submission_order, 1);
        assert_eq!(fresh.presenter_generation, 3);
        assert_eq!(host.submitted_capture_omissions(), 0);
    });
}

#[test]
fn stopping_admission_drains_in_submission_order_while_rendering_continues() {
    pollster::block_on(async {
        let (mut host, output, _instance) =
            Box::pin(host_and_output(wgpu::TextureFormat::Rgba8Unorm)).await;
        host.enable_submitted_capture(71).unwrap();
        let first = submit(&mut host, &output).submission_order;
        submit(&mut host, &output);
        let frame = image(&mut host);
        assert_eq!(frame.sample.submission_order, first);
        assert_eq!(frame.extent, [65, 37]);
        host.recycle_submitted_image(frame);
        submit(&mut host, &output); // Reuse an earlier ring slot while an older image is pending.
        host.stop_submitted_capture();
        submit(&mut host, &output);
        for order in [first + 1, first + 2] {
            let frame = image(&mut host);
            assert_eq!(frame.sample.submission_order, order);
            host.recycle_submitted_image(frame);
        }
        assert!(host.try_submitted_image().unwrap().is_none());
        assert_eq!(host.submitted_capture_usage(), Some((0, 0)));
        assert_eq!(host.submitted_capture_omissions(), 0);
    });
}
