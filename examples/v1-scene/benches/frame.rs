//! Integrated 10k simulation, extraction, upload, encode and GPU completion.
//! Deliberately serial and unpaced; the CLI separately exercises 60 Hz pacing.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use thrum_render::timing::GpuTimings;
use v1_scene::FrameHost;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

fn measure(bencher: divan::Bencher<'_, '_>, gpu_timestamps: bool) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("real native adapter required for this explicit benchmark");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: if gpu_timestamps {
            GpuTimings::requested_features(&adapter)
        } else {
            wgpu::Features::empty()
        },
        ..Default::default()
    }))
    .expect("device request succeeds");
    let mut host = pollster::block_on(FrameHost::new(&device, &queue, None))
        .expect("the prepared V1 workload must fit the adapter");
    let device_ref = &device;
    let mut now = Duration::ZERO;
    bencher.bench_local(move || {
        now += Duration::from_nanos(16_666_667);
        let frame = host
            .drive(now, None)
            .expect("the measured fixed-clock host must keep progressing");
        assert!(frame.submitted, "one update gets a free view slot");
        let complete = loop {
            if let Some(sample) = host.poll_completed().expect("GPU progress") {
                break sample;
            }
            device_ref
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .expect("GPU completion");
        };
        divan::black_box(complete);
    });
}

#[divan::bench]
fn assembled_frame_with_gpu_timestamps(bencher: divan::Bencher<'_, '_>) {
    measure(bencher, true);
}

#[divan::bench]
fn assembled_frame_without_gpu_timestamps(bencher: divan::Bencher<'_, '_>) {
    measure(bencher, false);
}
