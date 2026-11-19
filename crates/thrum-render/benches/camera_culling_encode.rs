//! CPU encode cost and heap allocations of one prepared frame, Direct versus
//! Frustum camera culling. A measured baseline for G3c, not a gate. GPU cost
//! is in `camera_culling`.
#![cfg(not(target_arch = "wasm32"))]

use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::synthetic::{
    CullMode, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticScene,
    SyntheticView,
};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const GENERATION: u64 = 1;

fn main() {
    divan::main();
}

#[expect(
    clippy::print_stdout,
    reason = "the benchmark reports actual mode selection before measuring"
)]
fn encode(
    test_gpu: &thrum_test_support::Gpu,
    bencher: divan::Bencher<'_, '_>,
    count: u32,
    culling: CullMode,
) {
    pollster::block_on(async {
        let adapter = test_gpu.adapter.clone();
        let (device, queue) = test_gpu.portable(Some(wgpu::Features::empty()));
        let mut scene = SyntheticScene::grid(count).expect("grid");
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, 1.374],
            [0.0, 0.0, 0.5],
            [0.0, 1.0, 0.0],
            1.0,
            16.0 / 9.0,
            0.01,
            10.0,
        )
        .expect("view");
        scene.sun_shadows = true;
        scene.camera_culling = culling;
        let capabilities = DeviceCapabilities::capture(&adapter, &device, GENERATION);
        let prepared = PreparedSynthetic::prepare_with_capabilities(
            &capabilities,
            &queue,
            &scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            1.0,
        )
        .await
        .expect("prepare");
        assert_eq!(prepared.camera_culling(), culling);
        println!("count={count} selection={:?}", prepared.culling_selection());
        let target = OffscreenTarget::new(&device, &prepared, 1280, 720).expect("target");
        bencher
            .with_inputs(|| {
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
            })
            .bench_local_values(|mut encoder| {
                prepared
                    .encode(&mut encoder, GENERATION, &target, None)
                    .expect("encode");
                encoder
            });
    });
}

#[divan::bench(args = [10_000, 100_000])]
fn direct(bencher: divan::Bencher<'_, '_>, count: u32) {
    let Some(test_gpu) = thrum_test_support::gpu("direct", None) else {
        return;
    };
    encode(&test_gpu, bencher, count, CullMode::Direct);
}

#[divan::bench(args = [10_000, 100_000])]
fn frustum(bencher: divan::Bencher<'_, '_>, count: u32) {
    let Some(test_gpu) = thrum_test_support::gpu("frustum", None) else {
        return;
    };
    encode(&test_gpu, bencher, count, CullMode::Frustum);
}
