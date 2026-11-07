//! Representative headless workload and link-conditioning microbenchmarks.
#[expect(
    dead_code,
    reason = "shared workload fixture has consumers across test and benchmark targets"
)]
#[path = "../tests/support/v1_workload.rs"]
mod support;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

#[divan::bench]
fn moving_world_and_four_peers(bencher: divan::Bencher<'_, '_>) {
    let mut workload = support::Workload::new();
    bencher.bench_local(move || {
        workload.step();
        divan::black_box(&workload);
    });
}

#[divan::bench]
fn four_peer_conditioner(bencher: divan::Bencher<'_, '_>) {
    let mut link = support::LinkConditioner::new(0xA0A0_2026_1103);
    let mut tick = 0;
    bencher.bench_local(move || {
        tick += 1;
        link.step(tick);
        divan::black_box(link.delivered);
    });
}
