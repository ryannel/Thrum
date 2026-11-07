//! Paired Direct-versus-Frustum GPU cost of the game's dense field along its
//! scripted flythrough. Both hosts step the same ticks; the order alternates
//! per tick (ABBA), and each frame completes before the next is driven.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use thrum_render::{capabilities::DeviceCapabilities, synthetic::CullMode};
use v1_game::{FrameConfig, FrameHost, FrameSample, GameContent, GameSetup, field_in_view_share};

const TICK_NS: u64 = 16_666_667;
const EXTENT: [u32; 2] = [1280, 720];
const WARMUP_TICKS: u64 = 60;
const TICKS: u64 = 1_800;
/// `gpu_pass_ns` index of forward (includes the camera visibility pass).
const FORWARD: usize = 3;
/// `gpu_pass_ns` index of tone, excluded from the whole-frame sum.
const TONE: usize = 5;

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
        let mut hosts = Vec::new();
        for camera_culling in [CullMode::Direct, CullMode::Frustum] {
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
                        content: GameContent::Field,
                        camera_culling,
                    },
                )
                .await
                .expect("prepared field"),
            );
        }
        // [mode][metric]: whole, forward.
        let mut rows: [[Vec<f64>; 2]; 2] = Default::default();
        let mut deltas = Vec::new();
        let mut share = 0.0;
        for tick in 1..=WARMUP_TICKS + TICKS {
            let now = Duration::from_nanos(tick * TICK_NS + 1);
            let order = if tick % 2 == 0 { [0, 1] } else { [1, 0] };
            let mut whole = [0.0; 2];
            for index in order {
                let report = hosts[index].drive(now, None).expect("field tick");
                assert!(
                    report.submitted && report.tick == tick,
                    "one frame per tick"
                );
                let sample = complete(&mut hosts[index], &device);
                let passes = sample.gpu_pass_ns.expect("timed passes");
                whole[index] = passes
                    .iter()
                    .enumerate()
                    .filter(|(slot, _)| *slot != TONE)
                    .map(|(_, ns)| ns / 1e6)
                    .sum();
                if tick > WARMUP_TICKS {
                    rows[index][0].push(whole[index]);
                    rows[index][1].push(passes[FORWARD] / 1e6);
                }
            }
            if tick > WARMUP_TICKS {
                deltas.push(whole[1] - whole[0]);
                if tick % 30 == 0 {
                    share += field_in_view_share(tick, EXTENT).expect("view");
                }
            }
        }
        #[expect(clippy::cast_precision_loss, reason = "small count")]
        let share = share / (TICKS / 30) as f64;
        println!(
            "adapter={}; field {} instances; {}x{}; shadows on; {TICKS} ticks after {WARMUP_TICKS} warm-up; \
             alternating order; mean in-view centres {:.1}%",
            adapter.get_info().name,
            GameContent::Field.entity_count(),
            EXTENT[0],
            EXTENT[1],
            share * 100.0
        );
        let [direct, frustum] = &mut rows;
        println!(
            "whole GPU ms p50 (p95) D→F: {:.3} ({:.3}) → {:.3} ({:.3}); forward p50 (p95) D→F: {:.3} ({:.3}) → {:.3} ({:.3}); paired F−D median {:+.3} ms",
            quantile(&mut direct[0], 0.5),
            quantile(&mut direct[0], 0.95),
            quantile(&mut frustum[0], 0.5),
            quantile(&mut frustum[0], 0.95),
            quantile(&mut direct[1], 0.5),
            quantile(&mut direct[1], 0.95),
            quantile(&mut frustum[1], 0.5),
            quantile(&mut frustum[1], 0.95),
            quantile(&mut deltas, 0.5),
        );
    });
}
