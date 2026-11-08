//! The dense flythrough field: Frustum and Occlusion camera culling must each
//! match the Direct oracle byte for byte along the scripted path, through snap
//! turns, cuts, near-plane crossings and drifting props. Set
//! `THRUM_FIELD_FULL_PATH=1` to compare every tick through the first cut, and
//! `THRUM_FIELD_SEQUENCE=<dir>` to write each culled mode's frames within 40
//! ticks of each event, under `<dir>/frustum/` and `<dir>/occlusion/`, for
//! motion review.
//! Run it with `--release` to reach the cut; debug builds cover the first turn.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use thrum_render::{capabilities::DeviceCapabilities, synthetic::CullMode};
use v1_game::{FrameConfig, FrameHost, GameContent, GameSetup};

const TICK_NS: u64 = 16_666_667;
/// Ticks 1 to 1,930 of the 3,600-tick loop: three snap turns and the first
/// quarter-loop cut (tick 1801). The next turn starts at tick 2101.
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
fn field_culling_matches_direct_along_the_flythrough() {
    fly(false);
}

/// The mixed field: props as a rock, a column and a spruce, every fifth
/// mirrored. Frustum on the bucket route matches bucket-ordered Direct; the
/// sequence goes under `<dir>/meshes-frustum/`.
#[test]
fn mixed_mesh_field_frustum_matches_bucket_direct_along_the_flythrough() {
    fly(true);
}

#[expect(
    clippy::print_stderr,
    reason = "the flythrough reports its culling counts"
)]
fn fly(field_meshes: bool) {
    let full = std::env::var_os("THRUM_FIELD_FULL_PATH").is_some();
    let sequence = std::env::var_os("THRUM_FIELD_SEQUENCE").map(std::path::PathBuf::from);
    // Occlusion keeps the cube until its history follows slot identity.
    let culled: &[(CullMode, &str)] = if field_meshes {
        &[(CullMode::Frustum, "meshes-frustum")]
    } else {
        &[
            (CullMode::Frustum, "frustum"),
            (CullMode::Occlusion, "occlusion"),
        ]
    };
    if let Some(directory) = &sequence {
        for (_, mode) in culled {
            std::fs::create_dir_all(directory.join(mode)).expect("sequence directory");
        }
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
        let modes = std::iter::once(CullMode::Direct).chain(culled.iter().map(|(mode, _)| *mode));
        for camera_culling in modes {
            hosts.push(
                FrameHost::new_with_setup(
                    &device,
                    &queue,
                    &capabilities,
                    FrameConfig::default(),
                    GameSetup {
                        content: GameContent::Field,
                        camera_culling,
                        valley_stars: false,
                        route_overlay: v1_game::RouteOverlay::Off,
                        field_meshes,
                    },
                )
                .await
                .expect("prepared field"),
            );
            // The renderer itself, not the setup, must run the asked mode.
            assert_eq!(
                hosts.last().map(FrameHost::camera_culling),
                Some(camera_culling)
            );
        }
        // The Frustum host's read-back draw counts: the fewest drawn and the
        // active slots then, so the comparison cannot pass unculled.
        let mut fewest: Option<(u32, u32)> = None;
        // Counts never hold a frame, so one may retire before its count.
        let mut uncounted = 0;
        // Ticks where both culled hosts were counted, and of those, where
        // Occlusion drew at most half of Frustum's list.
        let (mut paired, mut halved) = (0_u32, 0_u32);
        let extent = [640, 360];
        let mut compared = 0;
        for tick in 1..=LAST_TICK {
            let now = Duration::from_nanos(tick * TICK_NS + 1);
            let (mut frustum_drawn, mut occlusion_drawn) = (None, None);
            for host in &mut hosts {
                let report = host.drive(now, None).expect("field tick");
                assert_eq!(report.tick, tick, "one tick per callback");
                assert!(report.submitted, "every tick submits, so history matches");
                // Wait for each frame, so both hosts submit the same sequence.
                let sample = loop {
                    if let Some(sample) = host.poll_completed().expect("GPU progress") {
                        break sample;
                    }
                    device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: Some(Duration::from_secs(10)),
                        })
                        .expect("GPU wait");
                };
                match (host.camera_culling(), sample.camera_culling) {
                    (CullMode::Direct, None) => {}
                    (CullMode::Frustum, Some(counts)) => {
                        assert_eq!(counts.mode, CullMode::Frustum);
                        assert!(counts.drawn <= counts.active, "tick {tick}: {counts:?}");
                        if fewest.is_none_or(|(drawn, _)| counts.drawn < drawn) {
                            fewest = Some((counts.drawn, counts.active));
                        }
                        frustum_drawn = Some(counts.drawn);
                    }
                    (CullMode::Occlusion, Some(counts)) => {
                        assert_eq!(counts.mode, CullMode::Occlusion);
                        assert!(counts.drawn <= counts.active, "tick {tick}: {counts:?}");
                        occlusion_drawn = Some(counts.drawn);
                    }
                    (CullMode::Frustum | CullMode::Occlusion, None) => uncounted += 1,
                    (mode, counts) => panic!("tick {tick}: {mode:?} reported {counts:?}"),
                }
            }
            // The final list is the frustum list minus slots behind real
            // depth, so it can never exceed it.
            if let (Some(frustum), Some(occlusion)) = (frustum_drawn, occlusion_drawn) {
                assert!(
                    occlusion <= frustum,
                    "tick {tick}: Occlusion drew {occlusion} > Frustum {frustum}"
                );
                paired += 1;
                halved += u32::from(occlusion * 2 <= frustum);
            }
            if !sampled(tick, full) {
                continue;
            }
            let direct = hosts[0].capture_pixels().expect("Direct capture");
            assert_eq!(direct.len(), (extent[0] * extent[1] * 4) as usize);
            let near_event = EVENTS.iter().any(|event| tick.abs_diff(*event) <= 40);
            for (host, (_, mode)) in hosts[1..].iter_mut().zip(culled) {
                let culled = host.capture_pixels().expect("culled capture");
                if direct != culled {
                    let differing = direct
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(culled.as_chunks::<4>().0)
                        .filter(|(a, b)| a != b)
                        .count();
                    panic!("{mode} differs from Direct at tick {tick}: {differing} pixels");
                }
                if let Some(directory) = sequence.as_ref().filter(|_| near_event) {
                    let path = directory.join(mode).join(format!("{tick:05}.ppm"));
                    write_ppm(&path, &culled, extent);
                }
            }
            compared += 1;
        }
        let expected = if cfg!(debug_assertions) { 20 } else { 75 };
        assert!(compared >= expected, "{compared} frames compared");
        let (drawn, active) = fewest.expect("Frustum counts");
        eprintln!(
            "{compared} frames compared; Frustum drew {drawn} of {active} at its fewest; \
             {uncounted} culled frames retired before their count; Occlusion drew at most half \
             of Frustum on {halved} of {paired} paired ticks"
        );
        // It hides work: equality cannot pass unculled.
        assert!(
            field_meshes || (paired > 0 && halved * 10 >= paired * 9),
            "Occlusion halved Frustum's list on only {halved} of {paired} ticks"
        );
        assert!(
            drawn > 0 && drawn * 2 < active,
            "Frustum drew {drawn} of {active} at its fewest"
        );
    });
}
