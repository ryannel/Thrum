use thrum_g2_storage_comparison::{Hecs, Soa, Storage};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

fn tick<S: Storage>(bencher: divan::Bencher, count: usize) {
    let mut store = S::new(count);
    bencher.bench_local(move || {
        store.tick();
        divan::black_box(&store);
    });
}

#[divan::bench(args = [10_000, 100_000])]
fn hecs_tick(bencher: divan::Bencher, count: usize) {
    tick::<Hecs>(bencher, count);
}

#[divan::bench(args = [10_000, 100_000])]
fn soa_tick(bencher: divan::Bencher, count: usize) {
    tick::<Soa>(bencher, count);
}
