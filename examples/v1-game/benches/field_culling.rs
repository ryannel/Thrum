//! Paired Direct, Frustum and Occlusion GPU cost of the game's dense field
//! along its scripted flythrough. All hosts step the same ticks; the order
//! rotates per tick, and each frame completes before the next is driven.
//! Occlusion is also paired against Frustum, split by cover: ticks where its
//! final list is at most half of Frustum's are behind cover, the rest open.
//! Its whole frame includes the prepass list, depth prepass, pyramid and
//! final cull, which fold into the forward bucket. Runs at the game's
//! 1920x1080. On a shared machine, each 300-tick segment also reports its
//! paired medians beside the system load at its end, and the run asserts
//! Occlusion hid work on some ticks so an inert history cannot pass.
//! `THRUM_FIELD_MESHES=on` runs all three modes on rock, column and spruce props.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use thrum_render::{capabilities::DeviceCapabilities, synthetic::CullMode};
use v1_game::{FrameConfig, FrameHost, FrameSample, GameContent, GameSetup, field_in_view_share};

const TICK_NS: u64 = 16_666_667;
const EXTENT: [u32; 2] = [1920, 1080];
/// Ticks per reported segment, each with its load.
const SEGMENT: u64 = 300;
const WARMUP_TICKS: u64 = 60;
const TICKS: u64 = 1_800;
fn complete(host: &mut FrameHost, device: &wgpu::Device) -> FrameSample {
    loop {
        if let Some(sample) = host.poll_completed().expect("GPU progress") {
            return sample;
        }
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("GPU wait");
    }
}

/// The 1-minute load average, as reported by the OS (`?` if unavailable).
fn load() -> String {
    std::process::Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|text| text.split_whitespace().nth(1).map(str::to_owned))
        .unwrap_or_else(|| "?".to_owned())
}

fn quantile(values: &mut [f64], q: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "bounded sample index"
    )]
    let index = ((values.len() - 1) as f64 * q).round() as usize;
    values[index]
}

fn assert_live_history(prepasses: u64, hiding: u64) {
    assert!(
        prepasses > 0 && hiding > 0,
        "history prepass must run and hide slots"
    );
}

#[expect(clippy::print_stdout, reason = "benchmark segment report")]
fn report_segments(segments: &[(u64, f64, f64, String)]) {
    for (tick, frustum, occlusion, load) in segments {
        println!(
            "segment to tick {tick}: paired F−D median {frustum:+.3} ms, O−F median {occlusion:+.3} ms, load {load}"
        );
    }
}

#[expect(clippy::print_stdout, reason = "benchmark pass report")]
fn report_passes(modes: &[CullMode], passes: &mut [[Vec<f64>; 6]]) {
    for (mode, passes) in modes.iter().zip(passes) {
        let medians: Vec<String> = passes
            .iter_mut()
            .map(|values| format!("{:.3}", quantile(values, 0.5)))
            .collect();
        println!(
            "{mode:?} pass p50 ms (near, far+heights, clusters, forward, history, tone): {}",
            medians.join(", ")
        );
    }
}

#[expect(
    clippy::print_stdout,
    reason = "the explicit device benchmark reports paired GPU rows"
)]
fn main() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real native adapter required");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("timestamp-capable device");
        let capabilities =
            DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
        let field_meshes = std::env::var("THRUM_FIELD_MESHES").is_ok_and(|value| value == "on");
        let modes: &[CullMode] = &[CullMode::Direct, CullMode::Frustum, CullMode::Occlusion];
        let occlusion = modes.len() == 3;
        let mut hosts = Vec::new();
        for &camera_culling in modes {
            hosts.push(
                FrameHost::new_with_setup(
                    &device,
                    &queue,
                    &capabilities,
                    FrameConfig {
                        render_extent: EXTENT,
                        surface: None,
                    },
                    GameSetup {
                        display_overlay: thrum_render::synthetic::DisplayOverlay::Off,
                        diagnostic_clouds: false,
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
            assert_eq!(
                hosts.last().expect("host").camera_culling(),
                camera_culling,
                "resolved benchmark mode"
            );
        }
        // [mode][metric]: whole, forward. Deltas: Frustum − Direct, Occlusion − Direct.
        let mut rows: Vec<[Vec<f64>; 2]> = modes.iter().map(|_| Default::default()).collect();
        // [mode][pass] ms: near cascade, far cascade with the height layers,
        // local clusters, forward, history, tone.
        let mut per_pass: Vec<[Vec<f64>; 6]> = modes.iter().map(|_| Default::default()).collect();
        let mut deltas: [Vec<f64>; 2] = Default::default();
        // Occlusion − Frustum whole-frame ms: [behind cover, open].
        let mut against_frustum: [Vec<f64>; 2] = Default::default();
        let mut share = 0.0;
        // Per segment: (F−D, O−F) whole-frame ms, and the load at its end.
        let mut segment: [Vec<f64>; 2] = Default::default();
        let mut segments = Vec::new();
        let mut nonempty_prepasses = 0;
        let mut hiding_ticks = 0;
        for tick in 1..=WARMUP_TICKS + TICKS {
            let now = Duration::from_nanos(tick * TICK_NS + 1);
            // Rotate the order each tick, so no mode always runs first.
            let count = modes.len();
            let first = usize::try_from(tick).expect("small") % count;
            let mut whole = [0.0; 3];
            let mut drawn = [0_u32; 3];
            for index in (0..count).map(|step| (first + step) % count) {
                let report = hosts[index].drive(now, None).expect("field tick");
                assert!(
                    report.submitted && report.tick == tick,
                    "one frame per tick"
                );
                let sample = complete(&mut hosts[index], &device);
                let passes = sample.gpu_pass_ns.as_ref().expect("timed passes");
                drawn[index] = sample.camera_culling.map_or(0, |counts| counts.drawn);
                if index == 2 && tick > WARMUP_TICKS {
                    nonempty_prepasses += u64::from(
                        sample
                            .camera_culling
                            .is_some_and(|counts| counts.prepass > 0),
                    );
                }
                whole[index] = passes.sum() / 1e6;
                if tick > WARMUP_TICKS {
                    rows[index][0].push(whole[index]);
                    rows[index][1].push(passes.forward() / 1e6);
                    for (values, ns) in per_pass[index].iter_mut().zip([
                        passes.shadow_near(),
                        passes.shadow_far(),
                        passes.clusters(),
                        passes.forward(),
                        passes.history(),
                        passes.tone(),
                    ]) {
                        values.push(ns / 1e6);
                    }
                }
            }
            if tick > WARMUP_TICKS {
                segment[0].push(whole[1] - whole[0]);
                if occlusion {
                    segment[1].push(whole[2] - whole[1]);
                }
                if (tick - WARMUP_TICKS).is_multiple_of(SEGMENT) {
                    segments.push((
                        tick,
                        quantile(&mut segment[0], 0.5),
                        if occlusion {
                            quantile(&mut segment[1], 0.5)
                        } else {
                            f64::NAN
                        },
                        load(),
                    ));
                    segment = Default::default();
                }
                deltas[0].push(whole[1] - whole[0]);
                if occlusion {
                    deltas[1].push(whole[2] - whole[0]);
                }
                // A count can retire after its frame; such ticks are left out.
                if occlusion && drawn[1] > 0 && drawn[2] > 0 {
                    let open = usize::from(drawn[2] * 2 > drawn[1]);
                    hiding_ticks += u64::from(drawn[2] < drawn[1]);
                    against_frustum[open].push(whole[2] - whole[1]);
                }
                if tick % 30 == 0 {
                    share += field_in_view_share(tick, EXTENT).expect("view");
                }
            }
        }
        #[expect(clippy::cast_precision_loss, reason = "small count")]
        let share = share / (TICKS / 30) as f64;
        println!(
            "adapter={}; field {} instances, meshes {field_meshes}; {}x{}; shadows on; {TICKS} ticks after {WARMUP_TICKS} warm-up; \
             rotating order, shared machine; mean in-view centres {:.1}%",
            adapter.get_info().name,
            GameContent::Field.entity_count(),
            EXTENT[0],
            EXTENT[1],
            share * 100.0
        );
        report_segments(&segments);
        assert!(
            !occlusion || !against_frustum[0].is_empty(),
            "Occlusion hid at least half of Frustum's list on some counted tick"
        );
        assert_live_history(nonempty_prepasses, hiding_ticks);
        println!(
            "contended=true; nonempty_prepasses={nonempty_prepasses}; hiding_ticks={hiding_ticks}; whole includes every timed scene pass"
        );
        report_passes(modes, &mut per_pass);
        let [direct, culled @ ..] = rows.as_mut_slice() else {
            unreachable!("Direct always runs")
        };
        let direct = [
            (
                quantile(&mut direct[0], 0.5),
                quantile(&mut direct[0], 0.95),
            ),
            (
                quantile(&mut direct[1], 0.5),
                quantile(&mut direct[1], 0.95),
            ),
        ];
        for ((label, mode), delta) in ["F", "O"]
            .into_iter()
            .zip(culled.iter_mut())
            .zip(&mut deltas)
        {
            println!(
                "whole GPU ms p50 (p95) D→{label}: {:.3} ({:.3}) → {:.3} ({:.3}); forward p50 (p95) D→{label}: {:.3} ({:.3}) → {:.3} ({:.3}); paired {label}−D median {:+.3} ms",
                direct[0].0,
                direct[0].1,
                quantile(&mut mode[0], 0.5),
                quantile(&mut mode[0], 0.95),
                direct[1].0,
                direct[1].1,
                quantile(&mut mode[1], 0.5),
                quantile(&mut mode[1], 0.95),
                quantile(delta, 0.5),
            );
        }
        for (label, values) in ["behind cover", "open"]
            .into_iter()
            .zip(&mut against_frustum)
        {
            if values.is_empty() {
                println!("paired O−F {label}: no counted ticks");
                continue;
            }
            println!(
                "paired O−F {label}: {} ticks, median {:+.3} ms, p95 {:+.3} ms",
                values.len(),
                quantile(values, 0.5),
                quantile(values, 0.95),
            );
        }
    });
}
