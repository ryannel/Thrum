//! Complete no-socket four-peer link and 10k authoritative world diagnostic.
#![cfg(not(target_arch = "wasm32"))]

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

#[divan::bench]
fn four_peer_600_tick_run(bencher: divan::Bencher<'_, '_>) {
    bencher.bench_local(|| {
        divan::black_box(v1_scene::conditioned::run().expect("conditioned V1 run"));
    });
}
