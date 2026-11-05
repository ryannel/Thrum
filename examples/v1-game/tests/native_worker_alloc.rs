//! Metal measurement of allocations on both sides of the A3 handoff.
#![cfg(not(target_arch = "wasm32"))]
#![allow(
    unsafe_code,
    reason = "test-only counting allocator delegates every operation to System"
)]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use thrum_render::present::SurfaceProfile;
use v1_game::{FrameHost, SurfaceOutput};

const MEASURED_FRAMES: usize = 600;
static METAL_TEST_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn count(size: usize) {
    let _ = TRACK.try_with(|track| {
        if track.get() {
            let _ = ALLOCS.try_with(|value| value.set(value.get() + 1));
            let _ = BYTES.try_with(|value| value.set(value.get() + size as u64));
        }
    });
}

// SAFETY: The allocator forwards every operation and layout unchanged to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        // SAFETY: The layout is forwarded unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        // SAFETY: The layout is forwarded unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count(new_size);
        // SAFETY: The pointer and layout came from System; the new size is forwarded unchanged.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout came from System.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[test]
fn bounded_native_handoff_reports_main_and_worker_allocation_cost() {
    let _metal_guard = METAL_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native Metal adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let mut host = FrameHost::new(&device, &queue, v1_game::FrameConfig::default())
            .await
            .expect("prepared V1 scene");
        let wakes = Arc::new(AtomicU64::new(0));
        let worker_allocs = Arc::new(AtomicU64::new(0));
        let worker_bytes = Arc::new(AtomicU64::new(0));
        let wake_count = Arc::clone(&wakes);
        let allocation_count = Arc::clone(&worker_allocs);
        let byte_count = Arc::clone(&worker_bytes);
        host.enable_native_worker(move || {
            allocation_count.store(ALLOCS.with(Cell::get), Ordering::Release);
            byte_count.store(BYTES.with(Cell::get), Ordering::Release);
            TRACK.with(|track| track.set(true));
            wake_count.fetch_add(1, Ordering::Release);
        })
        .expect("worker");

        let mut dispatch_allocs = Vec::with_capacity(MEASURED_FRAMES);
        let mut dispatch_bytes = Vec::with_capacity(MEASURED_FRAMES);
        let mut extraction_allocs = Vec::with_capacity(MEASURED_FRAMES);
        let mut extraction_bytes = Vec::with_capacity(MEASURED_FRAMES);
        let mut previous_worker_allocs = 0;
        let mut previous_worker_bytes = 0;
        for tick in 1..=MEASURED_FRAMES as u64 + 1 {
            let now = Duration::from_nanos(tick * 16_666_667);
            ALLOCS.with(|count| count.set(0));
            BYTES.with(|count| count.set(0));
            TRACK.with(|track| track.set(true));
            let dispatch = host
                .drive(now, None)
                .expect("world tick and worker dispatch");
            TRACK.with(|track| track.set(false));
            let main_allocs = ALLOCS.with(Cell::get);
            let main_bytes = BYTES.with(Cell::get);
            assert_eq!(dispatch.completed_ticks, 1);
            assert!(!dispatch.submitted);
            let deadline = Instant::now() + Duration::from_secs(2);
            while wakes.load(Ordering::Acquire) < tick {
                assert!(Instant::now() < deadline, "worker did not complete");
                std::thread::yield_now();
            }
            let worker_allocs_now = worker_allocs.load(Ordering::Acquire);
            let worker_bytes_now = worker_bytes.load(Ordering::Acquire);
            if tick > 1 {
                dispatch_allocs.push(main_allocs);
                dispatch_bytes.push(main_bytes);
                extraction_allocs.push(worker_allocs_now - previous_worker_allocs);
                extraction_bytes.push(worker_bytes_now - previous_worker_bytes);
            }
            previous_worker_allocs = worker_allocs_now;
            previous_worker_bytes = worker_bytes_now;
            let submitted = host.drive(now, None).expect("zero-tick ready submission");
            assert!(submitted.submitted);
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                if host.poll_completed().expect("GPU progress").is_some() {
                    break;
                }
                assert!(Instant::now() < deadline, "GPU completion timed out");
                if let Err(error) = device.poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_millis(100)),
                }) {
                    assert!(
                        matches!(error, wgpu::PollError::Timeout),
                        "GPU wait: {error}"
                    );
                }
            }
        }
        assert_eq!(dispatch_allocs.len(), MEASURED_FRAMES);
        assert_eq!(extraction_allocs.len(), MEASURED_FRAMES);
        assert_eq!(host.dropped_views(), 0);
        assert!(
            extraction_allocs.iter().all(|count| *count == 0),
            "warmed worker extraction must reuse its CPU columns; first nonzero={:?}",
            extraction_allocs
                .iter()
                .enumerate()
                .find(|(_, count)| **count != 0)
        );
        assert!(
            dispatch_allocs.iter().all(|count| *count <= 4),
            "one bounded task dispatch must not fan out allocations"
        );
        let max = |values: &[u64]| values.iter().copied().max().expect("600 measured frames");
        println!(
            "A3 10k/1.5k Metal {MEASURED_FRAMES} warm frames: dispatch allocations median/max={}/{}, bytes median/max={}/{}; worker allocations median/max={}/{}, bytes median/max={}/{}",
            median(&dispatch_allocs),
            max(&dispatch_allocs),
            median(&dispatch_bytes),
            max(&dispatch_bytes),
            median(&extraction_allocs),
            max(&extraction_allocs),
            median(&extraction_bytes),
            max(&extraction_bytes),
        );
    });
}

#[test]
fn late_surface_submit_reports_main_thread_allocation_cost() {
    let _metal_guard = METAL_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native Metal adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features:
                    thrum_render::capabilities::DeviceCapabilities::requested_timing_features(
                        &adapter,
                    ),
                ..Default::default()
            })
            .await
            .expect("device");
        let extent = [640, 360];
        let mut host = FrameHost::new(
            &device,
            &queue,
            v1_game::FrameConfig {
                surface: Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent,
                }),
                ..Default::default()
            },
        )
        .await
        .expect("prepared V1 scene");
        let surface = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("late submission allocation target"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = surface.create_view(&wgpu::TextureViewDescriptor::default());
        let output = SurfaceOutput {
            view: &view,
            extent,
        };
        let mut allocations = Vec::with_capacity(MEASURED_FRAMES);
        let mut bytes = Vec::with_capacity(MEASURED_FRAMES);
        let mut advance_allocations = Vec::with_capacity(MEASURED_FRAMES);
        let mut encode_allocations = Vec::with_capacity(MEASURED_FRAMES);
        for tick in 1..=MEASURED_FRAMES as u64 + 1 {
            ALLOCS.with(|count| count.set(0));
            TRACK.with(|track| track.set(true));
            let advanced = host
                .advance(Duration::from_nanos(tick * 16_666_667))
                .expect("admitted tick");
            let advanced_allocs = ALLOCS.with(Cell::get);
            ALLOCS.with(|count| count.set(0));
            let encoded = advanced
                .encode_scene()
                .expect("encoded scene before drawable");
            let encoded_allocs = ALLOCS.with(Cell::get);
            TRACK.with(|track| track.set(false));
            ALLOCS.with(|count| count.set(0));
            BYTES.with(|count| count.set(0));
            TRACK.with(|track| track.set(true));
            let report = encoded
                .submit(Some(&output))
                .expect("late present submits once");
            TRACK.with(|track| track.set(false));
            assert!(report.submitted && report.surface_presented);
            if tick > 1 {
                allocations.push(ALLOCS.with(Cell::get));
                bytes.push(BYTES.with(Cell::get));
                advance_allocations.push(advanced_allocs);
                encode_allocations.push(encoded_allocs);
            }
            while host.poll_completed().expect("GPU progress").is_none() {
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(10)),
                    })
                    .expect("GPU completion");
            }
        }
        let max = |values: &[u64]| values.iter().copied().max().expect("measured frames");
        println!(
            "G3b late-surface submit {MEASURED_FRAMES} warm frames: advance alloc median/max={}/{}; scene encode alloc median/max={}/{}; submit alloc median/max={}/{}, bytes median/max={}/{}",
            median(&advance_allocations),
            max(&advance_allocations),
            median(&encode_allocations),
            max(&encode_allocations),
            median(&allocations),
            max(&allocations),
            median(&bytes),
            max(&bytes),
        );
    });
}

#[test]
fn autonomous_world_and_render_callback_report_warm_allocations() {
    let _metal_guard = METAL_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native Metal adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let extent = [640, 360];
        let host = FrameHost::new(
            &device,
            &queue,
            v1_game::FrameConfig {
                surface: Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent,
                }),
                ..Default::default()
            },
        )
        .await
        .expect("prepared V1 scene");
        let world_wakes = Arc::new(AtomicU64::new(0));
        let world_allocations = Arc::new(AtomicU64::new(0));
        let world_bytes = Arc::new(AtomicU64::new(0));
        let wake_count = Arc::clone(&world_wakes);
        let allocation_count = Arc::clone(&world_allocations);
        let byte_count = Arc::clone(&world_bytes);
        let origin = Instant::now();
        let (mut render, _controls) = host
            .into_threaded(origin, move || {
                let completed = wake_count.fetch_add(1, Ordering::AcqRel) + 1;
                if completed == 5 {
                    ALLOCS.with(|count| count.set(0));
                    BYTES.with(|count| count.set(0));
                    TRACK.with(|track| track.set(true));
                }
                if completed > 5 {
                    allocation_count.store(ALLOCS.with(Cell::get), Ordering::Release);
                    byte_count.store(BYTES.with(Cell::get), Ordering::Release);
                }
            })
            .expect("resident world");
        let surface = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threaded allocation target"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = surface.create_view(&wgpu::TextureViewDescriptor::default());
        let output = SurfaceOutput {
            view: &view,
            extent,
        };
        let mut render_allocations = Vec::with_capacity(100);
        let mut render_bytes = Vec::with_capacity(100);
        let mut submitted = 0;
        for callback in 1_u32..=120 {
            let target = origin + Duration::from_nanos(8_333_333).saturating_mul(callback);
            if let Some(delay) = target.checked_duration_since(Instant::now()) {
                std::thread::sleep(delay);
            }
            ALLOCS.with(|count| count.set(0));
            BYTES.with(|count| count.set(0));
            TRACK.with(|track| track.set(true));
            let report = render
                .drive(origin.elapsed(), Some(&output))
                .expect("threaded render callback");
            submitted += usize::from(report.submitted);
            while render.poll_completed().expect("GPU progress").is_some() {}
            TRACK.with(|track| track.set(false));
            if callback > 20 && report.submitted {
                render_allocations.push(ALLOCS.with(Cell::get));
                render_bytes.push(BYTES.with(Cell::get));
            }
        }
        assert!(
            render_allocations.len() >= 10,
            "at least ten warmed submitted frames are needed for allocation samples; submitted={submitted}/120"
        );
        assert!(world_wakes.load(Ordering::Acquire) >= 10);
        let max = |values: &[u64]| values.iter().copied().max().expect("measured frames");
        println!(
            "G3b threaded 120 callbacks (100 warm opportunities): submitted={submitted}/120, sampled_submitted={}; render allocations median/max={}/{}, bytes median/max={}/{}; world allocations/bytes after wake 5={}/{}",
            render_allocations.len(),
            median(&render_allocations),
            max(&render_allocations),
            median(&render_bytes),
            max(&render_bytes),
            world_allocations.load(Ordering::Acquire),
            world_bytes.load(Ordering::Acquire),
        );
    });
}

fn median(values: &[u64]) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}
