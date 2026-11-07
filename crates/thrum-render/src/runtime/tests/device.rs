use super::*;
use crate::{atmosphere::AtmosphereFrame, present::SurfaceProfile, synthetic::CullMode};
use std::{
    cell::Cell,
    sync::{Arc, atomic::Ordering},
};

#[derive(Default)]
struct Policy {
    camera: f32,
    atmosphere: bool,
    empty: bool,
    last_override: Cell<bool>,
}

impl SceneSource<SourceTick> for Policy {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        if self.empty {
            into.extract(*page, [0.0; 3], |_| {})
        } else {
            StaticScene::default().extract(into, page)
        }
    }

    fn frame(
        &self,
        _source: SourceTick,
        _alpha: f32,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        self.last_override.set(lighting.is_some());
        let view = SyntheticView::perspective(
            [self.camera, 0.0, 2.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            extent[0] as f32 / extent[1] as f32,
            0.1,
            20.0,
        )
        .map_err(|error| format!("test camera: {error:?}"))?;
        let lighting = lighting.unwrap_or_else(|| {
            if self.atmosphere {
                FrameLighting::Atmosphere(
                    AtmosphereFrame::new([0.0, 1.0, 0.0], 127_000.0, [1.0; 3], 2.0, 1)
                        .expect("atmosphere"),
                )
            } else {
                FrameLighting::Diagnostic(SyntheticLight::default())
            }
        });
        Ok(SceneFrame { view, lighting })
    }
}

type Host = FrameHost<StaticWorld, Policy>;

pub(super) async fn gpu(
    instance: &wgpu::Instance,
    generation: u64,
) -> (DeviceCapabilities, wgpu::Queue) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("recovery adapter");
    #[cfg(target_os = "macos")]
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_timing_features(&adapter),
            ..Default::default()
        })
        .await
        .expect("recovery device");
    (
        DeviceCapabilities::capture(&adapter, &device, generation),
        queue,
    )
}

async fn host(capabilities: &DeviceCapabilities, queue: &wgpu::Queue, policy: Policy) -> Host {
    let shadows = !policy.empty;
    culled_host(capabilities, queue, policy, CullMode::Direct, shadows).await
}

async fn culled_host(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    policy: Policy,
    camera_culling: CullMode,
    sun_shadows: bool,
) -> Host {
    let source = SourceTick {
        epoch: 5,
        tick: 0,
        timeline: 3,
    };
    let mut scene = SyntheticScene::grid(1).expect("scene");
    scene.sun_shadows = sun_shadows;
    scene.camera_culling = camera_culling;
    FrameHost::new(
        capabilities,
        queue,
        FrameConfig {
            render_extent: [64, 64],
            surface: Some(surface()),
        },
        StaticWorld {
            authority: source,
            publication: source,
        },
        policy,
        scene,
        0.125,
    )
    .await
    .expect("host")
}

pub(super) fn surface() -> SurfaceProfile {
    SurfaceProfile {
        format: wgpu::TextureFormat::Rgba8Unorm,
        extent: [64, 64],
    }
}

#[test]
fn a_missing_surface_binding_has_one_drop_and_one_scene_commit_on_both_routes() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let profile = surface();
        let texture = capabilities
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("unbound drawable fixture"),
                size: wgpu::Extent3d {
                    width: profile.extent[0],
                    height: profile.extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: profile.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
        let output = super::super::SurfaceOutput {
            view: &texture.create_view(&wgpu::TextureViewDescriptor::default()),
            extent: profile.extent,
        };
        for late in [false, true] {
            let mut host = host(&capabilities, &queue, Policy::default()).await;
            host.slots[0].presentation = None;
            let report = if late {
                host.advance(Duration::ZERO)
                    .expect("admit")
                    .encode_scene()
                    .expect("record")
                    .submit(Some(&output))
            } else {
                host.drive(Duration::ZERO, Some(&output))
            }
            .expect("missing binding degrades");
            assert!(report.submitted);
            assert!(report.dropped_view);
            assert!(!report.surface_presented);
            assert_eq!(host.dropped_surface(), 1);
            assert_eq!(host.last_view_source, Some(host.world.source()));
            assert_eq!(host.pending_frames(), 1);
        }
    });
}

fn pending_slots(host: &mut Host) {
    let mut report = host
        .advance(Duration::ZERO)
        .expect("world observation")
        .submit(None)
        .expect("no optional view");
    // Encode both real slots without polling between them. GPU completion may
    // occur, but neither report/slot has been retired by the host at destruction.
    for slot in 0..IN_FLIGHT {
        let frame = host.extract_serial(Duration::ZERO, 0, 0).expect("extract");
        host.submit_extracted(slot, &mut report, None, frame, super::super::Instant::now())
            .expect("actual slot submission");
    }
    assert_eq!(host.pending_frames(), IN_FLIGHT);
}

/// Collect one timed report per in-flight slot and return each frame's pass buckets.
fn drain_timed(host: &mut Host) -> Vec<[f64; 11]> {
    drain_samples(host)
        .into_iter()
        .map(|sample| sample.gpu_pass_ns.expect("real Metal pass timestamps"))
        .collect()
}

/// Collect one timed report per in-flight slot, in submission order.
fn drain_samples(host: &mut Host) -> Vec<FrameSample> {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut frames = Vec::with_capacity(IN_FLIGHT);
    while frames.len() < IN_FLIGHT {
        if let Some(sample) = host.poll_completed().expect("poll Metal frame") {
            assert!(
                !sample.timing_fault,
                "Metal frame timestamp collection failed"
            );
            frames.push(sample);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Metal frame report timed out"
        );
        std::thread::yield_now();
    }
    assert_eq!(host.pending_frames(), 0);
    frames
}

#[test]
#[cfg(target_os = "macos")]
fn metal_frustum_timestamps_drain_populated_then_empty_prefixes() {
    // Without Sun shadows: this one-cube fixture's shadow passes return invalid
    // Metal stamps in FrameHost for Direct as well (pre-existing, reported to root).
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = culled_host(
            &capabilities,
            &queue,
            Policy::default(),
            CullMode::Frustum,
            false,
        )
        .await;
        assert_eq!(host.prepared.camera_culling(), CullMode::Frustum);
        pending_slots(&mut host);
        assert_eq!(host.prepared.active_instance_count(), 1);
        let populated = drain_samples(&mut host);
        for sample in &populated {
            let counts = sample.camera_culling.expect("frustum counts");
            assert_eq!(
                (counts.mode, counts.active, counts.drawn),
                (CullMode::Frustum, 1, 1)
            );
            assert_eq!(counts.rejected(), 0);
        }
        for passes in populated
            .iter()
            .map(|sample| sample.gpu_pass_ns.expect("timed"))
        {
            assert!(passes[3] > 0.0, "forward includes the visibility pass");
            assert!(passes[4] > 0.0, "the populated prefix publishes history");
        }
        // The next submissions on the same slots carry an empty active prefix:
        // visibility still dispatches its offsets pass and stays timed.
        host.source_mut().empty = true;
        pending_slots(&mut host);
        assert_eq!(host.prepared.active_instance_count(), 0);
        let empty = drain_samples(&mut host);
        for sample in &empty {
            let counts = sample.camera_culling.expect("frustum counts");
            assert_eq!(
                (counts.active, counts.drawn),
                (0, 0),
                "an empty prefix lists nothing"
            );
        }
        for passes in empty
            .iter()
            .map(|sample| sample.gpu_pass_ns.expect("timed"))
        {
            assert!(
                passes[3] > 0.0,
                "visibility and the sky-free forward pass are timed"
            );
            assert_eq!(passes[4], 0.0, "there is no transform history to publish");
        }
    });
}

fn capture(host: &mut Host) -> Vec<u8> {
    let mut pending = host.begin_capture().expect("capture");
    finish_capture(host, &mut pending)
}

#[test]
#[cfg(target_os = "macos")]
fn culled_modes_survive_surface_resize_and_device_replacement() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, queue) = gpu(&instance, 7).await;
        let (second, new_queue) = gpu(&instance, 8).await;
        let mut direct = culled_host(
            &second,
            &new_queue,
            Policy::default(),
            CullMode::Direct,
            false,
        )
        .await;
        let expected = capture(&mut direct);
        for mode in [CullMode::Frustum] {
            let mut host = culled_host(&first, &queue, Policy::default(), mode, false).await;
            pending_slots(&mut host);
            drain_timed(&mut host);
            host.resize_surface([96, 48]).expect("resize");
            assert_eq!(
                host.prepared.camera_culling(),
                mode,
                "{mode:?} after resize"
            );
            pending_slots(&mut host);
            for sample in drain_samples(&mut host) {
                let counts = sample.camera_culling.expect("culled frames report counts");
                assert_eq!(counts.mode, mode, "{mode:?}: counts after resize");
            }
            let mut candidate = host
                .device_prepare_inputs()
                .expect("inputs")
                .prepare(&second, &new_queue, Some(surface()))
                .await
                .expect("replacement");
            host.publish_device(&mut candidate).expect("publish");
            assert_eq!(host.generation, 8);
            assert_eq!(
                host.prepared.camera_culling(),
                mode,
                "{mode:?} after replacement"
            );
            // The replacement's slots time every pass of the mode's layout,
            // and its capture equals Direct on the same device.
            pending_slots(&mut host);
            for sample in drain_samples(&mut host) {
                let counts = sample.camera_culling.expect("culled frames report counts");
                assert_eq!(counts.mode, mode, "{mode:?}: counts after replacement");
            }
            assert_eq!(capture(&mut host), expected, "{mode:?} replacement image");
        }
    });
}

#[test]
#[cfg(target_os = "macos")]
fn metal_frame_timestamps_survive_an_empty_instance_set() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(
            &capabilities,
            &queue,
            Policy {
                empty: true,
                atmosphere: true,
                ..Policy::default()
            },
        )
        .await;
        pending_slots(&mut host);
        assert_eq!(host.prepared.active_instance_count(), 0);
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        for _ in 0..IN_FLIGHT {
            let sample = loop {
                if let Some(sample) = host.poll_completed().expect("poll Metal frame") {
                    break sample;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "Metal frame report timed out"
                );
                std::thread::yield_now();
            };
            assert!(
                !sample.timing_fault,
                "Metal frame timestamp collection failed"
            );
            assert!(sample.camera_culling.is_none(), "Direct lists nothing");
            let passes = sample.gpu_pass_ns.expect("real Metal pass timestamps");
            assert!(passes[3] > 0.0, "the sky still executes the forward pass");
            assert_eq!(passes[4], 0.0, "there is no transform history to publish");
        }
    });
}

fn packed_pixels(
    prepared: &mut crate::synthetic::PreparedSynthetic,
    scene: &SyntheticScene,
    lighting: FrameLighting,
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
) -> Vec<u8> {
    let device = capabilities.device();
    let mut target =
        crate::synthetic::OffscreenTarget::new(device, prepared, 64, 64).expect("packed target");
    prepared
        .update_frame(queue, &mut target, scene, lighting, 1.0)
        .expect("current view without instance upload");
    let mut pixels = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_with_readback(
            &mut encoder,
            capabilities.generation(),
            &target,
            None,
            &mut pixels,
        )
        .expect("packed render");
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("packed rendering complete");
    pixels
        .try_collect(capabilities.generation())
        .expect("packed pixels")
        .expect("complete pixels")
}

#[test]
fn gpu_device_replacement_abandons_slots_and_isolates_old_candidates_and_callbacks() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, queue) = gpu(&instance, 7).await;
        let mut host = host(&first, &queue, Policy::default()).await;
        let mut cooked = host
            .visual_prepare_inputs()
            .prepare(
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
                Some([3; 32]),
            )
            .await
            .expect("old cooked candidate");
        let inputs = host.device_prepare_inputs().expect("detached inputs");
        let competing = host.device_prepare_inputs().expect("competing inputs");
        pending_slots(&mut host);
        let old_done = host
            .slots
            .iter()
            .map(|slot| Arc::clone(&slot.render_done))
            .collect::<Vec<_>>();
        let order = host.next_submission_order;
        let dropped = host.dropped_reports;
        let owner = Arc::clone(&host.visual_owner);
        host.source_mut().camera = 0.7;
        assert!(
            Arc::ptr_eq(&owner, &host.visual_owner),
            "camera edits do not allocate/renew ownership tokens"
        );
        // Advance retained CPU columns after the detached snapshot, without
        // updating its GPU copies. Publication must reconcile this newer scene.
        host.world.authority.tick = 1;
        host.world.publication = host.world.authority;
        host.extract_serial(Duration::ZERO, 0, 0)
            .expect("new retained columns");
        host.scene
            .apply_render_columns(host.extractor.frame(), &mut host.changed_slots);
        let world = host.world.source();
        first.device().destroy();
        let (replacement, new_queue) = gpu(&instance, 8).await;
        let mut candidate = inputs
            .prepare(&replacement, &new_queue, Some(surface()))
            .await
            .expect("replacement resources");
        let mut stale = competing
            .prepare(&replacement, &new_queue, Some(surface()))
            .await
            .expect("competing resources");
        assert!(candidate.resize_surface([0, 64]).is_err());
        candidate
            .resize_surface([96, 48])
            .expect("resize during preparation");
        host.publish_device(&mut candidate)
            .expect("publish without polling destroyed device");
        assert_eq!(
            host.presenter
                .as_ref()
                .expect("published presenter")
                .extent(),
            [96, 48]
        );
        assert!(
            candidate.resize_surface([64, 64]).is_err(),
            "consumed candidate cannot resize"
        );
        assert_eq!(host.world.source(), world);
        assert_eq!(host.generation, 8);
        assert_eq!(host.device, *replacement.device());
        assert_eq!(host.next_submission_order, order);
        assert_eq!(host.dropped_reports, dropped + IN_FLIGHT as u64);
        assert_eq!(host.pending_frames(), 0);
        assert!(host.completed.is_empty());
        assert!(host.history_reset_pending && host.upload_reset_pending);
        assert!(host.extractor.frame().source().is_none());
        assert!(host.last_view_source.is_none());
        assert!(host.visual_lighting.is_none());
        assert!(
            !host.source.last_override.get(),
            "dynamic policy did not become a fixed snapshot override"
        );
        assert_eq!(host.visual_revision, None);
        assert_eq!(
            host.prepared.fallback_exposure().to_bits(),
            0.125_f32.to_bits()
        );
        let current = host
            .source
            .frame(world, 1.0, host.render_extent, None)
            .expect("current policy")
            .view;
        assert_eq!(
            host.scene.view.pack_view(&current, 0.125),
            current.pack_view(&current, 0.125)
        );
        for (old, new) in old_done.iter().zip(&host.slots) {
            assert!(!Arc::ptr_eq(old, &new.render_done));
            old.store(true, Ordering::Release); // Delivery to an old callback mailbox.
            assert!(!new.render_done.load(Ordering::Acquire));
        }
        assert!(
            host.publish_visual(&mut cooked).is_err(),
            "old-device cooked candidate cannot install"
        );
        assert!(
            host.publish_device(&mut stale).is_err(),
            "competing base generation is stale"
        );
        assert!(
            host.publish_device(&mut candidate).is_err(),
            "consumed candidate cannot install twice"
        );
        // Render the installed GPU table before any subsequent extraction or
        // upload can repair it, and compare with a fresh current-scene prepare.
        let scene = host.scene.clone();
        let lighting = host
            .source
            .frame(world, 1.0, host.render_extent, None)
            .expect("current lighting")
            .lighting;
        let actual = packed_pixels(
            &mut host.prepared,
            &scene,
            lighting,
            &replacement,
            &new_queue,
        );
        let mut reference = crate::synthetic::PreparedSynthetic::prepare_with_capabilities(
            &replacement,
            &new_queue,
            &scene,
            lighting,
            0.125,
        )
        .await
        .expect("fresh current-scene reference");
        let expected = packed_pixels(&mut reference, &scene, lighting, &replacement, &new_queue);
        assert_eq!(
            actual, expected,
            "cold installation contains every reconciled instance row"
        );
        let frame = host
            .extract_serial(Duration::ZERO, 0, 0)
            .expect("fresh extraction");
        assert!(!frame.reused_source);
        let mut report = host
            .advance(Duration::ZERO)
            .expect("new device poll")
            .submit(None)
            .expect("observation");
        let sample = host
            .prepare_extracted(0, &mut report, frame)
            .expect("fresh view")
            .expect("current source");
        assert_eq!(sample.sample.tick, world.tick);
        assert!(
            host.history_reset_pending,
            "preparation/capture does not commit submitted history"
        );
        let image = host.capture_pixels().expect("actual replacement rendering");
        assert!(
            image
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[..3] != [0; 3])
        );
    });
}

#[test]
fn gpu_device_replacement_refuses_wrong_host_mode_and_failed_setup_without_losing_inputs() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, queue) = gpu(&instance, 11).await;
        let mut a = host(&first, &queue, Policy::default()).await;
        let mut b = host(&first, &queue, Policy::default()).await;
        let (second, next_queue) = gpu(&instance, 12).await;
        assert!(
            a.device_prepare_inputs()
                .expect("inputs")
                .prepare(&first, &queue, None)
                .await
                .is_err()
        );
        let same_device = DeviceCapabilities::capture_device(first.device(), 12);
        assert!(
            a.device_prepare_inputs()
                .expect("inputs")
                .prepare(&same_device, &queue, None)
                .await
                .is_err()
        );
        let invalid = SurfaceProfile {
            extent: [0, 64],
            ..surface()
        };
        assert!(
            a.device_prepare_inputs()
                .expect("inputs")
                .prepare(&second, &next_queue, Some(invalid))
                .await
                .is_err()
        );
        assert_eq!(a.generation, 11);
        let mut candidate = a
            .device_prepare_inputs()
            .expect("retry retained CPU inputs")
            .prepare(&second, &next_queue, Some(surface()))
            .await
            .expect("valid replacement");
        assert!(
            b.publish_device(&mut candidate).is_err(),
            "wrong host rejected before mutation"
        );
        assert_eq!(b.generation, 11);
        let owner = Arc::clone(&a.visual_owner);
        a.source_mut().atmosphere = true;
        assert!(
            a.publish_device(&mut candidate).is_err(),
            "changed lighting closure refuses publication"
        );
        assert_eq!(a.generation, 11);
        assert!(
            Arc::ptr_eq(&owner, &a.visual_owner),
            "failed publish leaves token valid for retry"
        );
        a.source_mut().atmosphere = false;
        a.source_mut().camera = -0.5;
        a.publish_device(&mut candidate)
            .expect("same candidate retries after policy repair");
        assert_eq!(a.generation, 12);
        assert_eq!(a.world.source().epoch, 5);

        // A selected cooked override and its fallback bits survive GPU rebuild;
        // a later successful cooked publication invalidates a competing rebuild.
        let mut selected = a
            .visual_prepare_inputs()
            .prepare(
                FrameLighting::Diagnostic(SyntheticLight::default()),
                0.0625,
                Some([9; 32]),
            )
            .await
            .expect("selected visual");
        assert!(a.publish_visual(&mut selected).expect("select cooked view"));
        let (third, last_queue) = gpu(&instance, 13).await;
        let mut rebuilt = a
            .device_prepare_inputs()
            .expect("selected inputs")
            .prepare(&third, &last_queue, Some(surface()))
            .await
            .expect("selected replacement");
        a.publish_device(&mut rebuilt)
            .expect("preserve selected visual");
        assert!(a.visual_lighting.is_some());
        assert_eq!(a.visual_revision, Some([9; 32]));
        assert_eq!(
            a.prepared.fallback_exposure().to_bits(),
            0.0625_f32.to_bits()
        );
        assert!(a.source.last_override.get());
        let (fourth, final_queue) = gpu(&instance, 14).await;
        let mut obsolete = a
            .device_prepare_inputs()
            .expect("old selected inputs")
            .prepare(&fourth, &final_queue, Some(surface()))
            .await
            .expect("old selected replacement");
        let mut newer = a
            .visual_prepare_inputs()
            .prepare(
                FrameLighting::Diagnostic(SyntheticLight::default()),
                0.25,
                Some([10; 32]),
            )
            .await
            .expect("new selection");
        assert!(a.publish_visual(&mut newer).expect("new selected visual"));
        assert!(a.publish_device(&mut obsolete).is_err());
        assert_eq!(a.visual_revision, Some([10; 32]));
        assert_eq!(a.prepared.fallback_exposure().to_bits(), 0.25_f32.to_bits());
    });
}

fn finish_capture<W: FrameWorld, V: SceneSource<W::Page>>(
    host: &mut FrameHost<W, V>,
    pending: &mut crate::runtime::PendingCapture,
) -> Vec<u8> {
    for _ in 0..1_000 {
        if let Some(pixels) = host.try_finish_capture(pending).expect("capture progress") {
            return pixels;
        }
        match host.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_millis(10)),
        }) {
            Ok(_) | Err(wgpu::PollError::Timeout) => {}
            Err(error) => panic!("capture wait: {error:?}"),
        }
    }
    panic!("capture never completed");
}

#[test]
fn gpu_pending_capture_is_caller_owned_and_retired_with_its_generation() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, queue) = gpu(&instance, 7).await;
        let mut host = host(&first, &queue, Policy::default()).await;
        let mut early = host.begin_capture().expect("first capture");
        let mut late = host.begin_capture().expect("second capture");
        assert_eq!(
            (early.tick(), early.extent(), early.generation()),
            (late.tick(), late.extent(), late.generation()),
            "both describe the same committed endpoint"
        );
        // Each token owns its readback: finishing out of order works.
        let second = finish_capture(&mut host, &mut late);
        let first_image = finish_capture(&mut host, &mut early);
        assert_eq!(first_image, second);
        assert_eq!(
            host.capture_pixels().expect("waiting capture"),
            first_image,
            "native capture is the same operation with a wait"
        );
        let mut stale = host.begin_capture().expect("old generation capture");
        let inputs = host.device_prepare_inputs().expect("detached inputs");
        first.device().destroy();
        let (replacement, new_queue) = gpu(&instance, 8).await;
        let mut candidate = inputs
            .prepare(&replacement, &new_queue, Some(surface()))
            .await
            .expect("replacement resources");
        host.publish_device(&mut candidate)
            .expect("publish replacement");
        assert!(
            host.try_finish_capture(&mut stale).is_err(),
            "a retired generation terminates its capture"
        );
    });
}

#[test]
fn gpu_pending_capture_is_refused_by_another_device_with_the_same_generation() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, first_queue) = gpu(&instance, 7).await;
        let (second, second_queue) = gpu(&instance, 7).await;
        let mut recorder = host(&first, &first_queue, Policy::default()).await;
        let mut other = host(&second, &second_queue, Policy::default()).await;
        let mut pending = recorder.begin_capture().expect("capture");
        assert_eq!(pending.generation(), 7);
        // Let the recorder's readback complete, so a wrong host would find
        // pixels ready if it collected without checking the device.
        recorder
            .wait(pending.submitted.clone(), "capture")
            .expect("recorder wait");
        assert!(
            other.try_finish_capture(&mut pending).is_err(),
            "another device with the same generation refuses the capture"
        );
        assert!(
            recorder
                .try_finish_capture(&mut pending)
                .expect("recorder finish")
                .is_some(),
            "the refusal leaves the capture intact for its own host"
        );
    });
}
