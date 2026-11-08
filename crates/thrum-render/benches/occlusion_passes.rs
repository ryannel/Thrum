//! Paired Frustum-versus-Occlusion GPU cost of the camera draw, pass by pass,
//! on 10k and 100k cubes: open (top-down, nothing hidden) and behind cover
//! (a grazing view behind a wall). ABBA blocks; every frame updates and then
//! commits its view, as a host does, so Occlusion's history advances and its
//! runs are settled. Each sample reads the frame's culling counts and the run
//! asserts Occlusion's prepass replays a non-empty set and, behind cover, its
//! final list hides at least 30% of Frustum's; a bench whose history stops
//! advancing fails instead of timing Frustum's work twice. GPU timestamps
//! exclude waits. Occlusion's cost is its prepass list, depth prepass, pyramid
//! and final cull, against the work its smaller final list saves in forward.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::synthetic::{
    CullMode, CullingCounts, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight,
    SyntheticScene, SyntheticView,
};
use thrum_render::timing::GpuTimings;

const GENERATION: u64 = 1;
const WARMUP_BLOCKS: usize = 10;
const BLOCKS: usize = 60;
const EXTENT: [u32; 2] = [1280, 720];
/// Culling passes, then forward, as named in the timestamp report.
const PASSES: [&str; 6] = [
    "camera visibility",
    "camera prepass list",
    "camera depth prepass",
    "camera depth pyramid",
    "camera final cull",
    "synthetic forward opaque",
];

fn wait(device: &wgpu::Device, submitted: wgpu::SubmissionIndex) {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("GPU finishes");
}

/// One updated, submitted and committed frame: whole GPU ms (tone excluded),
/// each of `PASSES`, and the frame's culling counts.
fn sample(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    (prepared, scene): (&mut PreparedSynthetic, &SyntheticScene),
    target: &mut OffscreenTarget,
    timings: &mut GpuTimings,
) -> (f64, [f64; 6], CullingCounts) {
    prepared
        .update_frame(
            queue,
            target,
            scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            1.0,
        )
        .expect("frame updates");
    target.arm_culling_stats();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, GENERATION, target, Some(timings))
        .expect("scene encodes");
    timings
        .finish_render(GENERATION, &encoder)
        .expect("timestamp batch closes");
    wait(device, queue.submit([encoder.finish()]));
    target.map_culling_stats(prepared.camera_culling(), prepared.active_instance_count());
    prepared.commit_frame();
    let mut resolve = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    assert!(
        timings
            .resolve_if_ready(GENERATION, &mut resolve)
            .expect("resolve ready")
    );
    wait(device, queue.submit([resolve.finish()]));
    let passes = timings
        .try_collect(GENERATION)
        .expect("valid timestamps")
        .expect("completed timestamps");
    let named = PASSES.map(|name| {
        passes
            .iter()
            .find(|pass| pass.name.as_ref() == name)
            .map_or(0.0, |pass| pass.nanoseconds / 1_000_000.0)
    });
    let whole = passes
        .iter()
        .filter(|pass| pass.name.as_ref() != "synthetic tone encode")
        .map(|pass| pass.nanoseconds / 1_000_000.0)
        .sum();
    // The resolve wait polled the device, so the count mapping has settled.
    let counts = target
        .take_culling_counts()
        .expect("every frame reports its culling counts");
    (whole, named, counts)
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

/// Slot 0 becomes a wall across the grazing view, in front of most of the grid:
/// the scene of `the_grazing_view_hides_slots_and_replays_its_visible_set`,
/// which asserts Occlusion hides at least 30% of 10k slots. The wall is slot 0,
/// so Frustum's slot-ordered forward already draws it first and early depth
/// rejects most hidden fragments; the saving is vertex work, small on cubes.
fn wall(scene: &mut SyntheticScene) {
    let wall = &mut scene.instances[0];
    let half = [0.005, 0.3, 0.2];
    for (axis, extent) in half.into_iter().enumerate() {
        for row in 0..3 {
            let unit = if axis == row { 1.0 } else { 0.0 };
            wall.current[axis][row] = unit * extent;
            wall.normal[axis][row] = unit / extent;
        }
    }
    wall.current[3] = [0.0, 0.05, 0.55, 1.0];
    wall.previous = wall.current;
}

#[expect(
    clippy::print_stdout,
    reason = "the explicit device benchmark reports paired GPU samples"
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
        let capabilities = DeviceCapabilities::capture(&adapter, &device, GENERATION);
        let lighting = FrameLighting::Diagnostic(SyntheticLight::default());
        println!(
            "adapter={}; {}x{}; shadows on; ABBA {BLOCKS} blocks (A=Frustum, B=Occlusion), \
             view updated and committed every frame; medians in ms",
            adapter.get_info().name,
            EXTENT[0],
            EXTENT[1]
        );
        println!(
            "count | view | whole F→O | within-block O−F | F visibility | O prepass list | \
             O depth prepass | O pyramid | O final cull | forward F→O | drawn F→O (O prepass)"
        );
        let open = SyntheticView::perspective(
            [0.0, 0.0, 2.5],
            [0.0, 0.0, 0.5],
            [0.0, 1.0, 0.0],
            1.0,
            16.0 / 9.0,
            0.01,
            10.0,
        )
        .expect("open view");
        let cover = SyntheticView::perspective(
            [-0.2, 0.05, 0.6],
            [1.0, 0.0, 0.45],
            [0.0, 1.0, 0.0],
            1.0,
            16.0 / 9.0,
            0.01,
            10.0,
        )
        .expect("cover view");
        for count in [10_000, 100_000] {
            for (label, view) in [("open", open), ("behind cover", cover)] {
                let mut prepared = Vec::new();
                let mut scenes = Vec::new();
                for culling in [CullMode::Frustum, CullMode::Occlusion] {
                    let mut scene = SyntheticScene::grid(count).expect("grid");
                    scene.view = view;
                    scene.sun_shadows = true;
                    scene.camera_culling = culling;
                    if label == "behind cover" {
                        wall(&mut scene);
                    }
                    prepared.push(
                        PreparedSynthetic::prepare_with_capabilities(
                            &capabilities,
                            &queue,
                            &scene,
                            lighting,
                            1.0,
                        )
                        .await
                        .expect("prepare"),
                    );
                    scenes.push(scene);
                }
                let mut targets: Vec<_> = prepared
                    .iter()
                    .map(|p| {
                        OffscreenTarget::new(&device, p, EXTENT[0], EXTENT[1]).expect("target")
                    })
                    .collect();
                let mut timings = [0, 1].map(|index| {
                    GpuTimings::new(&capabilities, &queue, prepared[index].scene_pass_count())
                        .expect("timestamp batch")
                });
                assert!(timings[0].is_enabled(), "device timestamps required");
                let gpu = (&device, &queue);
                for _ in 0..WARMUP_BLOCKS {
                    for index in [0, 1, 1, 0] {
                        std::hint::black_box(sample(
                            gpu,
                            (&mut prepared[index], &scenes[index]),
                            &mut targets[index],
                            &mut timings[index],
                        ));
                    }
                }
                let mut whole = [Vec::new(), Vec::new()];
                let mut passes: [Vec<[f64; 6]>; 2] = [Vec::new(), Vec::new()];
                let mut drawn: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
                let mut replayed = Vec::new();
                let mut delta = Vec::new();
                for _ in 0..BLOCKS {
                    let mut block = [0.0; 4];
                    for (position, index) in [0, 1, 1, 0].into_iter().enumerate() {
                        let (total, named, counts) = sample(
                            gpu,
                            (&mut prepared[index], &scenes[index]),
                            &mut targets[index],
                            &mut timings[index],
                        );
                        block[position] = total;
                        whole[index].push(total);
                        passes[index].push(named);
                        drawn[index].push(f64::from(counts.drawn));
                        if index == 1 {
                            assert!(
                                counts.prepass > 0,
                                "{count} {label}: Occlusion's prepass replays last frame's set"
                            );
                            replayed.push(f64::from(counts.prepass));
                        }
                    }
                    delta.push(((block[1] + block[2]) - (block[0] + block[3])) * 0.5);
                }
                let pass = |index: usize, slot: usize| {
                    median(
                        &mut passes[index]
                            .iter()
                            .map(|named| named[slot])
                            .collect::<Vec<_>>(),
                    )
                };
                let drawn = drawn.map(|mut counts| median(&mut counts));
                if label == "behind cover" {
                    assert!(
                        drawn[1] * 10.0 < drawn[0] * 7.0,
                        "{count} {label}: Occlusion hides at least 30%: {drawn:?}"
                    );
                }
                println!(
                    "{count} | {label} | {:.3} → {:.3} | {:+.3} | {:.3} | {:.3} | {:.3} | {:.3} | \
                     {:.3} | {:.3} → {:.3} | {:.0} → {:.0} ({:.0})",
                    median(&mut whole[0]),
                    median(&mut whole[1]),
                    median(&mut delta),
                    pass(0, 0),
                    pass(1, 1),
                    pass(1, 2),
                    pass(1, 3),
                    pass(1, 4),
                    pass(0, 5),
                    pass(1, 5),
                    drawn[0],
                    drawn[1],
                    median(&mut replayed),
                );
            }
        }
    });
}
