//! A native capture whose readback completion another thread's poll delivers late.

use super::*;
use std::sync::atomic::AtomicBool;

/// Threads polling the host's device, each with a slow map callback of its own ahead of the
/// capture's in whichever poll collects both. A thread that only polls almost never wins the
/// race against the capture's own wait, so it would prove nothing.
const POLLERS: usize = 16;
/// How long each poller's own map callback takes.
const SLOW_CALLBACK: Duration = Duration::from_micros(500);
/// On the code before the fix, one to thirteen of 300 captures failed, in every run.
const CAPTURES: usize = 300;

/// Maps a small buffer of its own for writing (nothing is read) with a slow callback and polls
/// until it has run, until told to stop. A mapping that waits on no submission is ready at once,
/// so it is collected first.
fn slow_poller(device: wgpu::Device, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture race poller"),
            size: 16,
            usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        while !stop.load(Ordering::Relaxed) {
            let done = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&done);
            buffer.slice(..).map_async(wgpu::MapMode::Write, move |_| {
                std::thread::sleep(SLOW_CALLBACK);
                flag.store(true, Ordering::Release);
            });
            while !done.load(Ordering::Acquire) {
                device.poll(wgpu::PollType::Poll).ok();
            }
            buffer.unmap();
        }
    })
}

#[test]
fn gpu_capture_pixels_waits_for_a_readback_another_thread_delivers() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Some(test_gpu) = thrum_test_support::gpu_on(
            "gpu_capture_pixels_waits_for_a_readback_another_thread_delivers",
            &instance,
        ) else {
            return;
        };
        let (capabilities, queue) = gpu(&test_gpu, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        let reference = host.capture_pixels().expect("capture with no other poller");
        let stop = Arc::new(AtomicBool::new(false));
        let pollers: Vec<_> = (0..POLLERS)
            .map(|_| slow_poller(host.device.clone(), Arc::clone(&stop)))
            .collect();
        let mut failures = Vec::new();
        for index in 0..CAPTURES {
            match host.capture_pixels() {
                Ok(pixels) => assert_eq!(pixels, reference, "capture {index} differs"),
                Err(error) => failures.push((index, format!("{error:?}"))),
            }
        }
        stop.store(true, Ordering::Relaxed);
        for poller in pollers {
            poller.join().expect("poller");
        }
        assert!(
            failures.is_empty(),
            "{} of {CAPTURES} captures failed while {POLLERS} threads polled the device; first {:?}",
            failures.len(),
            failures.first()
        );
    });
}
