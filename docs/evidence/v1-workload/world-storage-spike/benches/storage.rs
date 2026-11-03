use thrum_world_storage_spike::{Adapter, Bevy, COUNT, Custom, Hecs, Row};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

#[divan::bench]
fn custom_tick(b: divan::Bencher) {
    let mut a = Custom::new();
    b.bench_local(move || {
        a.tick();
        divan::black_box(&a);
    });
}
#[divan::bench]
fn hecs_tick(b: divan::Bencher) {
    let mut a = Hecs::new();
    b.bench_local(move || {
        a.tick();
        divan::black_box(&a);
    });
}
#[divan::bench]
fn bevy_tick(b: divan::Bencher) {
    let mut a = Bevy::new();
    b.bench_local(move || {
        a.tick();
        divan::black_box(&a);
    });
}

fn snapshot<A: Adapter>(b: divan::Bencher) {
    let mut a = A::new();
    let mut out = vec![Row::default(); COUNT];
    b.bench_local(move || {
        a.copy_snapshot(&mut out);
        divan::black_box(&out);
    });
}
#[divan::bench]
fn custom_snapshot(b: divan::Bencher) {
    snapshot::<Custom>(b);
}
#[divan::bench]
fn hecs_snapshot(b: divan::Bencher) {
    snapshot::<Hecs>(b);
}
#[divan::bench]
fn bevy_snapshot(b: divan::Bencher) {
    snapshot::<Bevy>(b);
}

fn query<A: Adapter>(b: divan::Bencher) {
    let mut a = A::new();
    b.bench_local(move || {
        divan::black_box(a.query_checksum());
    });
}
#[divan::bench]
fn custom_query(b: divan::Bencher) {
    query::<Custom>(b);
}
#[divan::bench]
fn hecs_query(b: divan::Bencher) {
    query::<Hecs>(b);
}
#[divan::bench]
fn bevy_query(b: divan::Bencher) {
    query::<Bevy>(b);
}

fn changed<A: Adapter>(b: divan::Bencher) {
    let mut a = A::new();
    b.bench_local(move || {
        divan::black_box(a.sparse_change_probe());
    });
}
#[divan::bench]
fn custom_one_percent_changed(b: divan::Bencher) {
    changed::<Custom>(b);
}
#[divan::bench]
fn hecs_one_percent_changed(b: divan::Bencher) {
    changed::<Hecs>(b);
}
#[divan::bench]
fn bevy_one_percent_changed(b: divan::Bencher) {
    changed::<Bevy>(b);
}

fn sparse_transition<A: Adapter>(b: divan::Bencher) {
    let mut a = A::new();
    b.bench_local(move || {
        a.toggle_sparse_ownership();
        divan::black_box(&a);
    });
}
#[divan::bench]
fn custom_sparse_transition(b: divan::Bencher) {
    sparse_transition::<Custom>(b);
}
#[divan::bench]
fn hecs_sparse_transition(b: divan::Bencher) {
    sparse_transition::<Hecs>(b);
}
#[divan::bench]
fn bevy_sparse_transition(b: divan::Bencher) {
    sparse_transition::<Bevy>(b);
}

fn restore<A: Adapter>(b: divan::Bencher) {
    let mut a = A::new();
    let mut out = vec![Row::default(); COUNT];
    a.copy_snapshot(&mut out);
    b.bench_local(move || {
        a.restore(&out);
        divan::black_box(&a);
    });
}
#[divan::bench]
fn custom_restore(b: divan::Bencher) {
    restore::<Custom>(b);
}
#[divan::bench]
fn hecs_restore(b: divan::Bencher) {
    restore::<Hecs>(b);
}
#[divan::bench]
fn bevy_restore(b: divan::Bencher) {
    restore::<Bevy>(b);
}
