//! The dense flythrough field: Frustum camera culling must match the Direct
//! oracle byte for byte along the scripted path, through snap turns, cuts,
//! near-plane crossings and drifting props. Set `THRUM_FIELD_FULL_PATH=1` to
//! compare every tick through the first cut, and `THRUM_FIELD_SEQUENCE=<dir>`
//! to write the Frustum frames within 40 ticks of each event for motion review.
//! Run it with `--release` to reach the cut; debug builds cover the first turn.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use thrum_render::{capabilities::DeviceCapabilities, synthetic::CullMode};
use v1_game::{FrameConfig, FrameHost, GameContent, GameSetup};

const TICK_NS: u64 = 16_666_667;
/// Through the first quarter-loop cut (tick 1801) and the turn after it.
/// Unoptimised builds (`./dev check`) stop after the first snap turn, since
/// they drive both hosts about seven times slower.
const LAST_TICK: u64 = if cfg!(debug_assertions) { 460 } else { 1_930 };
/// Snap-turn starts and ends, and the cut, with their neighbours.
const EVENTS: [u64; 7] = [301, 421, 901, 1021, 1501, 1621, 1801];

fn sampled(tick: u64, full: bool) -> bool {
    full || tick % 30 == 1
        || EVENTS
            .iter()
            .any(|event| (event.saturating_sub(1)..=event + 1).contains(&tick))
}

fn write_ppm(path: &std::path::Path, rgba: &[u8], extent: [u32; 2]) {
    let mut ppm = format!("P6\n{} {}\n255\n", extent[0], extent[1]).into_bytes();
    for pixel in rgba.as_chunks::<4>().0 {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, ppm).expect("write field frame");
}

#[test]
fn field_frustum_matches_direct_along_the_flythrough() {
    let full = std::env::var_os("THRUM_FIELD_FULL_PATH").is_some();
    let sequence = std::env::var_os("THRUM_FIELD_SEQUENCE").map(std::path::PathBuf::from);
    if let Some(directory) = &sequence {
        std::fs::create_dir_all(directory).expect("sequence directory");
    }
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter required for the field comparison");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("device");
        let capabilities =
            DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
        let mut hosts = Vec::new();
        for camera_culling in [CullMode::Direct, CullMode::Frustum] {
            hosts.push(
                FrameHost::new_with_setup(
                    &device,
                    &queue,
                    &capabilities,
                    FrameConfig::default(),
                    GameSetup {
                        content: GameContent::Field,
                        camera_culling,
                    },
                )
                .await
                .expect("prepared field"),
            );
        }
        let extent = [640, 360];
        let mut compared = 0;
        for tick in 1..=LAST_TICK {
            let now = Duration::from_nanos(tick * TICK_NS + 1);
            for host in &mut hosts {
                let report = host.drive(now, None).expect("field tick");
                assert_eq!(report.tick, tick, "one tick per callback");
                assert!(report.submitted, "every tick submits, so history matches");
                // Wait for each frame, so both hosts submit the same sequence.
                while host.poll_completed().expect("GPU progress").is_none() {
                    device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: Some(Duration::from_secs(10)),
                        })
                        .expect("GPU wait");
                }
            }
            if !sampled(tick, full) {
                continue;
            }
            let direct = hosts[0].capture_pixels().expect("Direct capture");
            let frustum = hosts[1].capture_pixels().expect("Frustum capture");
            assert_eq!(direct.len(), (extent[0] * extent[1] * 4) as usize);
            if direct != frustum {
                let differing = direct
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(frustum.as_chunks::<4>().0)
                    .filter(|(a, b)| a != b)
                    .count();
                panic!("Frustum differs from Direct at tick {tick}: {differing} pixels");
            }
            let near_event = EVENTS.iter().any(|event| tick.abs_diff(*event) <= 40);
            if let Some(directory) = sequence.as_ref().filter(|_| near_event) {
                write_ppm(&directory.join(format!("{tick:05}.ppm")), &frustum, extent);
            }
            compared += 1;
        }
        let expected = if cfg!(debug_assertions) { 20 } else { 75 };
        assert!(compared >= expected, "{compared} frames compared");
    });
}
