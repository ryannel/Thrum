//! CPU encode cost and heap allocations of one prepared frame, Direct versus
//! Frustum camera culling. A measured baseline for G3c, not a gate. GPU cost
//! is in `camera_culling`.
#![cfg(not(target_arch = "wasm32"))]

use thrum_render::synthetic::{
    CullMode, OffscreenTarget, PreparedSynthetic, SyntheticScene, SyntheticView,
};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const GENERATION: u64 = 1;

fn main() {
    divan::main();
}

fn encode(bencher: divan::Bencher<'_, '_>, count: u32, culling: CullMode) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real native adapter required");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
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
        let prepared = PreparedSynthetic::prepare(&device, &queue, GENERATION, &scene)
            .await
            .expect("prepare");
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
    encode(bencher, count, CullMode::Direct);
}

#[divan::bench(args = [10_000, 100_000])]
fn frustum(bencher: divan::Bencher<'_, '_>, count: u32) {
    encode(bencher, count, CullMode::Frustum);
}
