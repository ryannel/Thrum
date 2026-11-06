//! Paired arithmetic cost for consuming one future display target.
//! This is not a substitute for the on-display G3b frame budget.

use std::{hint::black_box, time::Duration};

use thrum_render::frame::display_target_lead;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

#[divan::bench]
fn queued_time_baseline(bencher: divan::Bencher<'_, '_>) {
    bencher.bench(|| {
        let target = black_box(100.008_f64);
        let observed = black_box(100.0_f64);
        let seconds = target - observed;
        black_box(if seconds.is_finite() && seconds > 0.0 {
            Duration::from_secs_f64(seconds.min(0.05))
        } else {
            Duration::ZERO
        })
    });
}

#[divan::bench]
fn worker_time_classified(bencher: divan::Bencher<'_, '_>) {
    bencher.bench(|| {
        black_box(display_target_lead(
            black_box(100.008_f64),
            black_box(100.0_f64),
        ))
    });
}
