//! Metal timestamp limits, ownership through submission, and admission order.

#[cfg(target_os = "macos")]
pub mod ray_support;
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
mod metal {
    use std::{
        sync::{Mutex, PoisonError, mpsc},
        time::Duration,
    };

    use objc2::{rc::Retained, runtime::ProtocolObject};
    use objc2_metal::{MTLCommandBuffer, MTLCommandQueue, MTLDevice, MTLEvent, MTLSharedEvent};
    use thrum_render::{
        capabilities::DeviceCapabilities,
        error::FrameGpuError,
        governor::{DisplayRecord, ThermalState},
        present::SurfaceProfile,
        runtime::{FrameConfig, FrameError, FrameHost, FrameSample, SurfaceOutput},
        synthetic::{FrameLighting, RayBuildProbe, SyntheticLight, SyntheticScene},
        timing::{
            TimestampQueryRequestStage, TimingError, create_timestamp_query_set,
            with_timestamp_query_request_probe,
        },
    };
    use thrum_test_support::{Gpu, Profile, Restriction};
    use wgpu::util::DeviceExt;

    use super::{ray_support::support as rays, support};

    const DEADLINE: Duration = Duration::from_secs(5);
    const EXTENT: [u32; 2] = [16, 16];
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const EXHAUSTED: TimingError = TimingError::TimestampQuerySetsExhausted;
    const SLOTS: usize = 2;
    const METAL_LIMIT: usize = 32;

    // These tests deliberately exhaust a process-wide limit. Keep the other
    // timing tests in this binary out until each witness has released its devices.
    static TEST: Mutex<()> = Mutex::new(());

    fn device(gpu: &Gpu, generation: u64) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
        gpu.fresh_adapter()
            .device_with_hardware(Profile::Portable, &Restriction::default(), generation, &[])
            .expect("timing device")
    }

    fn host(
        caps: &DeviceCapabilities,
        queue: &wgpu::Queue,
        reconstruction: bool,
        ray_structure: bool,
    ) -> Result<support::Host, FrameError> {
        let mut scene = SyntheticScene::grid(1).expect("small scene");
        scene.view = scene.view.with_reconstruction(reconstruction);
        scene.ray_structure = ray_structure;
        let source = support::Source {
            view: scene.view,
            lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
            instances: vec![([0.0; 3], 0)],
            lights: Vec::new(),
        };
        pollster::block_on(Box::pin(FrameHost::new(
            caps,
            queue,
            FrameConfig {
                render_extent: EXTENT,
                surface: Some(SurfaceProfile {
                    format: FORMAT,
                    extent: EXTENT,
                }),
            },
            support::World,
            source,
            scene,
            1.0,
        )))
    }

    fn draw(host: &mut support::Host, device: &wgpu::Device, time: Duration) -> FrameSample {
        let view = support::drawable(device, FORMAT, EXTENT);
        let output = SurfaceOutput {
            view: &view,
            extent: EXTENT,
        };
        assert!(
            host.drive(time, Some(&output))
                .expect("drive frame")
                .surface_presented
        );
        frame_sample(host)
    }

    fn frame_sample(host: &mut support::Host) -> FrameSample {
        thrum_test_support::wait::until("frame timed out", DEADLINE, || {
            host.poll_completed().expect("poll frame")
        })
    }

    fn query(device: &wgpu::Device) -> Result<wgpu::QuerySet, TimingError> {
        create_timestamp_query_set(device, "Metal limit witness", 2)
    }

    fn hold(device: &wgpu::Device, count: usize) -> Vec<wgpu::QuerySet> {
        (0..count)
            .map(|_| query(device).expect("room for a timestamp query set"))
            .collect()
    }

    fn assert_host_refused(result: Result<support::Host, FrameError>) {
        let error = result.err().expect("timed host must be refused");
        assert!(
            matches!(
                error,
                FrameError::Gpu {
                    error: FrameGpuError::Timing(EXHAUSTED),
                    ..
                }
            ),
            "{error:?}"
        );
        assert!(error.to_string().contains("timestamp query sets"));
    }

    fn assert_timed(sample: &FrameSample) {
        assert!(sample.surface_presented && sample.timing_requested && !sample.timing_fault);
        let passes = sample.gpu_pass_ns.as_ref().expect("GPU pass times");
        assert!(passes.forward() > 0.0);
        assert!(
            passes
                .passes()
                .iter()
                .all(|pass| pass.nanoseconds.is_finite() && pass.nanoseconds >= 0.0)
        );
        assert!(sample.gpu_present_ns.is_some_and(|ns| ns > 0.0));
    }

    // Adds a test-only wait to this exact queue. Signal on teardown even when an
    // assertion fails, so the wgpu queue can drain before the device is destroyed.
    struct QueueGate(Retained<ProtocolObject<dyn MTLSharedEvent>>);

    impl QueueGate {
        #[expect(
            unsafe_code,
            reason = "Metal test gate appends an independent command buffer to the retained test queue"
        )]
        fn hold(queue: &wgpu::Queue) -> Self {
            // SAFETY: The test owns this queue and submits no wgpu work from
            // another thread. The wait takes no wgpu resource and owns its event.
            let metal = unsafe { queue.as_hal::<wgpu::hal::api::Metal>() }.expect("Metal queue");
            let raw = metal.as_raw();
            let event = raw.device().newSharedEvent().expect("shared event");
            let barrier = raw.commandBuffer().expect("native test barrier");
            barrier.encodeWaitForEvent_value(ProtocolObject::<dyn MTLEvent>::from_ref(&*event), 1);
            barrier.commit();
            Self(event)
        }
    }

    impl Drop for QueueGate {
        fn drop(&mut self) {
            self.0.setSignaledValue(1);
        }
    }

    pub(super) fn slot_reuse() {
        let _test = TEST.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(gpu) = rays::gate("governed slot timestamp readback") else {
            return;
        };
        let (device, queue, caps) = device(&gpu, 1);
        let mut host = host(&caps, &queue, true, false).expect("governed host");
        assert_eq!(
            device.get_internal_counters().hal.query_sets.read(),
            isize::try_from(SLOTS).expect("slot count fits")
        );
        assert!(host.enable_governor(DisplayRecord {
            interval: Duration::from_millis(50),
            thermal: ThermalState::Nominal,
        }));
        let view = support::drawable(&device, FORMAT, EXTENT);
        let output = SurfaceOutput {
            view: &view,
            extent: EXTENT,
        };
        assert!(
            host.drive(Duration::ZERO, Some(&output))
                .expect("first render")
                .submitted
        );
        let first = host.last_submission().expect("first order");
        rays::wait(&device);

        let gate = QueueGate::hold(&queue);
        // This poll queues the first resolve behind the gate and frees that slot.
        assert!(
            host.poll_completed()
                .expect("queue the first resolve")
                .is_none()
        );
        assert_eq!(host.timing_stats().pending, 1);
        assert!(
            host.drive(Duration::from_millis(16), Some(&output))
                .expect("reuse while mapping")
                .submitted
        );
        let second = host.last_submission().expect("second order");
        // Both render slots can now hold work even with the first resolve gated.
        assert!(
            host.drive(Duration::from_millis(32), Some(&output))
                .expect("other render slot")
                .submitted
        );
        let third = host.last_submission().expect("third order");
        assert_eq!(host.pending_frames(), 3);
        assert!(
            host.poll_completed()
                .expect("first resolve still held")
                .is_none()
        );
        assert_eq!(
            device.get_internal_counters().hal.query_sets.read(),
            isize::try_from(SLOTS).expect("slot count fits")
        );
        drop(gate);

        for order in [first, second, third] {
            let sample = frame_sample(&mut host);
            assert_eq!(sample.submission_order, order);
            assert!(sample.governor.is_some());
            assert_timed(&sample);
        }
        assert_eq!(host.pending_frames(), 0);
    }

    pub(super) fn process_limit() {
        let _test = TEST.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(gpu) = rays::gate("process-wide Metal timestamp limit") else {
            return;
        };
        let (left, left_queue, left_caps) = rays::device(&gpu, "ray-query", 1);
        let (right, right_queue, right_caps) = device(&gpu, 2);
        let mut left_host = host(&left_caps, &left_queue, false, true).expect("left host");
        let mut right_host = host(&right_caps, &right_queue, false, false).expect("right host");
        assert_timed(&draw(&mut left_host, &left, Duration::ZERO));
        assert_timed(&draw(&mut right_host, &right, Duration::ZERO));
        let observer = left_host
            .observe_ray_mesh_builds()
            .expect("installed ray build probe");

        let _left_sets = hold(&left, METAL_LIMIT / 2 - SLOTS);
        let _right_sets = hold(&right, METAL_LIMIT / 2 - SLOTS);
        for device in [&left, &right] {
            assert_eq!(device.get_internal_counters().hal.query_sets.read(), 16);
            assert_eq!(
                query(device).expect_err("process limit must refuse the next query"),
                EXHAUSTED
            );
        }
        assert_host_refused(host(&right_caps, &right_queue, false, false));
        assert_build_hits(&observer, &left, &left_queue);
        assert!(observer.take_recorded().is_empty());
        assert_timed(&draw(&mut left_host, &left, Duration::from_millis(16)));
        assert_timed(&draw(&mut right_host, &right, Duration::from_millis(16)));
    }

    fn assert_build_hits(observer: &RayBuildProbe, device: &wgpu::Device, queue: &wgpu::Queue) {
        let triangle = [[-0.5_f32, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]];
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ray build refusal triangle"),
            contents: bytemuck::cast_slice(&triangle),
            usage: wgpu::BufferUsages::BLAS_INPUT,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ray build refusal indices"),
            contents: bytemuck::cast_slice(&[0_u32, 1, 2]),
            usage: wgpu::BufferUsages::BLAS_INPUT,
        });
        let size = wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count: 3,
            index_format: Some(wgpu::IndexFormat::Uint32),
            index_count: Some(3),
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        };
        let blas = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: None,
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![size.clone()],
            },
        );
        let mut tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: None,
            max_instances: 1,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_BUILD,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        });
        tlas[0] = Some(wgpu::TlasInstance::new(
            &blas,
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            0,
            1,
        ));
        let probe = rays::Probe::bind(device, |layout| {
            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tlas.as_binding(),
                }],
            }))
        });
        let build = wgpu::BlasBuildEntry {
            blas: &blas,
            geometry: wgpu::BlasGeometries::TriangleGeometries(vec![wgpu::BlasTriangleGeometry {
                size: &size,
                vertex_buffer: &vertices,
                first_vertex: 0,
                vertex_stride: 12,
                index_buffer: Some(&indices),
                first_index: Some(0),
                transform_buffer: None,
                transform_buffer_offset: None,
            }]),
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        observer.encode(&mut encoder, "unobserved triangle".into(), &build);
        queue.submit([encoder.finish()]);
        rays::wait(device);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.build_acceleration_structures([], [&tlas]);
        let readback = probe.record(
            device,
            &mut encoder,
            &[[0.0, 0.0, -1.0, 0.0], [2.0, 0.0, -1.0, 0.0]],
        );
        let submission = queue.submit([encoder.finish()]);
        assert_eq!(rays::answers(device, readback, submission), [1, 0]);
    }

    pub(super) fn host_churn() {
        let _test = TEST.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(gpu) = rays::gate("timestamp places return when a host drops") else {
            return;
        };
        let (device, queue, caps) = device(&gpu, 1);
        for _ in 0..(64 / SLOTS) {
            let mut host = host(&caps, &queue, false, false).expect("prepare another timed host");
            assert_eq!(
                device.get_internal_counters().hal.query_sets.read(),
                isize::try_from(SLOTS).expect("slot count fits")
            );
            assert_timed(&draw(&mut host, &device, Duration::ZERO));
            drop(host);
            rays::wait(&device);
            assert_eq!(device.get_internal_counters().hal.query_sets.read(), 0);
        }
    }

    pub(super) fn retention() {
        let _test = TEST.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(gpu) = rays::gate("Metal timestamp release after collected submission") else {
            return;
        };
        let (device, queue, caps) = device(&gpu, 1);
        let mut held = hold(&device, METAL_LIMIT - 1);
        let recorded = query(&device).expect("last place");
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("retained timestamp resolve"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        drop(encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: &recorded,
                beginning_of_pass_write_index: Some(0),
                end_of_pass_write_index: Some(1),
            }),
        }));
        encoder.resolve_query_set(&recorded, 0..2, &buffer, 0);
        drop(recorded);
        assert_eq!(
            query(&device).expect_err("the existing query still occupies the last place"),
            EXHAUSTED
        );

        let gate = QueueGate::hold(&queue);
        let submission = queue.submit([encoder.finish()]);
        assert!(matches!(
            device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_millis(1)),
            }),
            Err(wgpu::PollError::Timeout)
        ));
        assert_eq!(
            query(&device).expect_err("the existing query still occupies the last place"),
            EXHAUSTED
        );
        drop(gate);
        rays::wait(&device);
        held.push(query(&device).expect("collection returned the place"));

        drop(held.pop().expect("release one place"));
        assert_host_refused(host(&caps, &queue, false, false));
        let last_set = query(&device).expect("the refused host left a place");
        drop(last_set);
        drop(held.pop().expect("release a second place"));
        let mut host = host(&caps, &queue, false, false).expect("room for the whole host");
        assert_timed(&draw(&mut host, &device, Duration::ZERO));
    }

    enum Order {
        HostFirst,
        SingleFirst,
    }

    fn contend<T: Send, U: Send>(
        first: impl FnOnce() -> T + Send,
        second: impl FnOnce() -> U + Send,
    ) -> (T, U) {
        std::thread::scope(|scope| {
            let (admitted, trial_complete) = mpsc::channel();
            let (release, resume) = mpsc::channel();
            let first = scope.spawn(move || {
                with_timestamp_query_request_probe(
                    move |stage| {
                        if stage == TimestampQueryRequestStage::Admitted {
                            admitted.send(()).expect("trial observer");
                            resume.recv_timeout(DEADLINE).expect("release trial");
                        }
                    },
                    first,
                )
            });
            trial_complete
                .recv_timeout(DEADLINE)
                .expect("first trial passed");
            let (arrived, waiting) = mpsc::channel();
            let (finished, answer) = mpsc::channel();
            scope.spawn(move || {
                let result = with_timestamp_query_request_probe(
                    move |stage| {
                        if stage == TimestampQueryRequestStage::Waiting {
                            arrived.send(()).expect("waiting observer");
                        }
                    },
                    second,
                );
                finished
                    .send(result)
                    .unwrap_or_else(|_| panic!("answer observer"));
            });
            waiting
                .recv_timeout(DEADLINE)
                .expect("second request has arrived");
            let blocked = answer.recv_timeout(Duration::from_millis(50));
            release.send(()).expect("first request held");
            assert!(matches!(blocked, Err(mpsc::RecvTimeoutError::Timeout)));
            (
                first.join().expect("first request"),
                answer.recv_timeout(DEADLINE).expect("second request"),
            )
        })
    }

    pub(super) fn contention() {
        let _test = TEST.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(gpu) = rays::gate("timestamp admission between Metal trial and wgpu allocation")
        else {
            return;
        };
        let (left, left_queue, left_caps) = device(&gpu, 1);
        let (right, right_queue, right_caps) = device(&gpu, 2);
        let mut left_survivor = host(&left_caps, &left_queue, false, false).expect("left host");
        let mut right_survivor = host(&right_caps, &right_queue, false, false).expect("right host");
        left_survivor.set_timing_requested(false);
        right_survivor.set_timing_requested(false);
        for (i, order) in [Order::HostFirst, Order::SingleFirst]
            .into_iter()
            .enumerate()
        {
            let _left_sets = hold(&left, METAL_LIMIT / 2 - 1 - SLOTS);
            let _right_sets = hold(&right, METAL_LIMIT / 2 - 1 - SLOTS);
            let prepare = || host(&left_caps, &left_queue, false, false);
            let single = || query(&right);
            match order {
                Order::HostFirst => {
                    let (prepared, single) = contend(prepare, single);
                    let mut prepared = prepared.expect("first trial admitted both frame slots");
                    assert_eq!(
                        single.expect_err("the host took the two remaining places"),
                        EXHAUSTED
                    );
                    assert_timed(&draw(&mut prepared, &left, Duration::ZERO));
                }
                Order::SingleFirst => {
                    let (single, prepared) = contend(single, prepare);
                    let _single = single.expect("first trial admitted the single request");
                    assert_host_refused(prepared);
                    let _last = query(&right).expect("one place after the all-or-nothing refusal");
                }
            }
            for (host, device) in [(&mut left_survivor, &left), (&mut right_survivor, &right)] {
                let sample = draw(host, device, Duration::from_millis(i as u64 * 16));
                assert!(
                    sample.surface_presented
                        && !sample.timing_requested
                        && sample.gpu_pass_ns.is_none()
                );
            }
        }
    }
}

thrum_test_support::macos_gpu_test! {
    fn one_query_set_for_each_frame_slot() { metal::slot_reuse(); }
}
thrum_test_support::macos_gpu_test! {
    fn the_33rd_across_two_devices_is_refused_and_both_survive() { metal::process_limit(); }
}
thrum_test_support::macos_gpu_test! {
    fn hosts_made_and_dropped_in_turn_never_reach_the_limit() { metal::host_churn(); }
}
thrum_test_support::macos_gpu_test! {
    fn a_place_returns_when_metal_has_it_and_a_host_needs_every_slot() { metal::retention(); }
}
thrum_test_support::macos_gpu_test! {
    fn two_requests_for_the_last_places_are_answered_in_turn() { metal::contention(); }
}
