//! Contended release measurements of the production structure recorder.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(feature = "creation-counters")]
mod metal {
    #[cfg(target_os = "macos")]
    use super::ray_support::support;

    #[cfg(target_os = "macos")]
    use std::io::Write as _;
    #[cfg(target_os = "macos")]
    use std::time::Instant;
    use thrum_render::capabilities::DeviceCapabilities;
    #[cfg(target_os = "macos")]
    use thrum_render::synthetic::{PreparedSynthetic, SyntheticScene};
    #[cfg(target_os = "macos")]
    use thrum_render::timing::GpuTimings;
    #[cfg(target_os = "macos")]
    use thrum_render::timing::TimingCategory;

    #[cfg(target_os = "macos")]
    const BEFORE: &str = "before structure build";
    #[cfg(target_os = "macos")]
    const AFTER: &str = "consume structure build";
    #[cfg(target_os = "macos")]
    const BUILD_SAMPLES: usize = 13;

    #[cfg(target_os = "macos")]
    struct Measurement<'a> {
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        caps: &'a DeviceCapabilities,
        marker: wgpu::ComputePipeline,
        marker_group: wgpu::BindGroup,
        timer: GpuTimings,
    }

    #[cfg(target_os = "macos")]
    impl<'a> Measurement<'a> {
        fn new(
            device: &'a wgpu::Device,
            queue: &'a wgpu::Queue,
            caps: &'a DeviceCapabilities,
        ) -> Self {
            let marker_source = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(BEFORE),
                source: wgpu::ShaderSource::Wgsl(
                    "@group(0) @binding(0) var<storage, read_write> sink: atomic<u32>;
                 @compute @workgroup_size(1) fn mark() { atomicAdd(&sink, 1u); }"
                        .into(),
                ),
            });
            let marker = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(BEFORE),
                layout: None,
                module: &marker_source,
                entry_point: Some("mark"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
            let sink = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timestamp marker sink"),
                size: 4,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let marker_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(BEFORE),
                layout: &marker.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sink.as_entire_binding(),
                }],
            });
            let timer = GpuTimings::new(caps, queue, 2).expect("timestamp batch");
            assert!(timer.is_enabled(), "Metal timestamp queries are required");
            Self {
                device,
                queue,
                caps,
                marker,
                marker_group,
                timer,
            }
        }

        fn gpu_span(
            &mut self,
            prepared: &PreparedSynthetic,
            probe: &support::Probe,
            build: bool,
        ) -> f64 {
            // Initial arena writes and successive pose uploads are flushed by the
            // coordinator. Timestamp resolution and polling are outside the span.
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let generation = self.caps.generation();
            {
                let mut marker = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(BEFORE),
                    timestamp_writes: self
                        .timer
                        .compute_writes(generation, BEFORE, TimingCategory::Transfer)
                        .expect("begin"),
                });
                marker.set_pipeline(&self.marker);
                marker.set_bind_group(0, &self.marker_group, &[]);
                marker.dispatch_workgroups(1, 1, 1);
            }
            if build {
                prepared
                    .encode_ray_structure_measurement(&mut encoder)
                    .expect("production recorder");
            }
            let _probe = probe.record_timed(
                self.device,
                &mut encoder,
                &[[0.01, 0.01, -4.0, 0.0]],
                self.timer
                    .compute_writes(generation, AFTER, TimingCategory::Transfer)
                    .expect("end"),
            );
            self.timer
                .finish_render(generation, &encoder)
                .expect("schedule timestamps");
            self.queue.submit([encoder.finish()]);
            support::wait(self.device);
            let mut resolve = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            assert!(
                self.timer
                    .resolve_if_ready(generation, &mut resolve)
                    .expect("resolve timestamps")
            );
            self.queue.submit([resolve.finish()]);
            support::wait(self.device);
            let consumer = self
                .timer
                .try_collect(generation)
                .expect("timestamps")
                .expect("ready")[1]
                .nanoseconds;
            let gpu_ns = self.timer.gap_between(BEFORE, AFTER).expect("bracket") + consumer;
            assert!(
                gpu_ns.is_finite() && gpu_ns > 0.0,
                "invalid Metal bracket: {gpu_ns}"
            );
            gpu_ns / 1_000.0
        }

        fn cpu_build(&self, prepared: &PreparedSynthetic) -> [f64; 2] {
            let start = Instant::now();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared
                .encode_ray_structure_measurement(&mut encoder)
                .expect("production recorder");
            let recorded = start.elapsed();
            self.queue.submit([encoder.finish()]);
            let submitted = start.elapsed().saturating_sub(recorded);
            support::wait(self.device);
            [recorded.as_secs_f64() * 1e6, submitted.as_secs_f64() * 1e6]
        }
    }

    thrum_test_support::macos_gpu_test! {
    #[ignore = "contended cost record is run by scripts/ray_tracing_cost.py"]
    fn report_structure_cost() {
        let Some(gpu) = support::gate("ray structure costs") else {
            return;
        };
        let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
        let mut measurement = Measurement::new(&device, &queue, &caps);
        {
            // Warm Metal's timing/consumer shaders against unrelated geometry,
            // leaving every measured scene's first BLAS and TLAS unbuilt.
            let mut warm = SyntheticScene::grid(64).expect("warm-up fixture");
            warm.ray_structure = true;
            let (warm, _target) = support::prepare(&device, &queue, &caps, &warm);
            queue.submit([]);
            support::wait(&device);
            let probe = support::Probe::new(&device, &warm);
            measurement.gpu_span(&warm, &probe, true);
        }
        for (count, movers) in [(10_000, 0), (10_000, 1_500), (10_000, 10_000), (100_513, 0)] {
            let mut scene = SyntheticScene::grid(count).expect("scene at the planned count");
            let slots: Vec<_> = (0..movers).collect();
            let (ordinary, ordinary_target) = support::prepare(&device, &queue, &caps, &scene);
            queue.submit([]);
            support::wait(&device);
            let memory = || caps.resident_device_bytes().expect("Metal allocated size");
            let plain_held = memory();
            let (second_plain, second_target) = support::prepare(&device, &queue, &caps, &scene);
            queue.submit([]);
            support::wait(&device);
            let plain_bytes = memory() - plain_held;
            drop((second_plain, second_target));
            support::wait(&device);

            scene.ray_structure = true;
            let before = memory();
            let (mut prepared, _target) = support::prepare(&device, &queue, &caps, &scene);
            queue.submit([]);
            support::wait(&device);
            let raw_bytes = memory() - before;
            let resident = raw_bytes
                .checked_sub(plain_bytes)
                .expect("ray allocation delta");
            assert!(resident > 0);

            let probe = support::Probe::new(&device, &prepared);
            let first_gpu = measurement.gpu_span(&prepared, &probe, true);
            // The graph retains its initial mesh list until a submitted frame
            // commits. Run the first-build CPU sample on a fresh scene to match it.
            let (cold, _target) = support::prepare(&device, &queue, &caps, &scene);
            queue.submit([]);
            support::wait(&device);
            let [first_record, first_submit] = measurement.cpu_build(&cold);
            drop(cold);
            prepared.commit_frame();
            let control_gpu = measurement.gpu_span(&prepared, &probe, false);
            let mut output = std::io::stdout().lock();
            writeln!(output, "structure {count} {movers} {plain_bytes} {raw_bytes} {resident} {first_record:.3} {first_submit:.3} {first_gpu:.3} {control_gpu:.3}").expect("report first build");
            for frame in 0..BUILD_SAMPLES {
                for slot in &mut scene.instances[..movers as usize] {
                    slot.previous = slot.current;
                    slot.current[3][2] += if frame % 2 == 0 { 0.0125 } else { -0.0125 };
                }
                prepared
                    .upload_instances(&queue, &scene.instances, &slots)
                    .expect("update movers");
                queue.submit([]);
                support::wait(&device);
                let [record, submit] = measurement.cpu_build(&prepared);
                let gpu = measurement.gpu_span(&prepared, &probe, true);
                writeln!(
                    output,
                    "rebuild {count} {movers} {frame} {record:.3} {submit:.3} {gpu:.3}"
                )
                .expect("report rebuild");
            }
            drop((ordinary, ordinary_target));
        }
    }
    }
}
