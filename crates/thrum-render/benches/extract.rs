//! A2 CPU render-publication cost with the selected 10k/1.5k workload.

use thrum_render::extract::{RenderExtractor, SourceTick};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

#[divan::bench]
fn a2_extract_ten_thousand(bencher: divan::Bencher<'_, '_>) {
    let mut extractor = RenderExtractor::new(10_000, 10_000);
    let mut tick = 0_u64;
    extractor.extract(
        SourceTick {
            epoch: 1,
            tick,
            timeline: 0,
        },
        [0.0; 3],
        |sink| {
            for slot in 0..10_000_u32 {
                sink.push(slot, 1, u128::from(slot), [0.0; 3], 1);
            }
        },
    );
    bencher.bench_local(move || {
        tick += 1;
        let report = extractor.extract(
            SourceTick {
                epoch: 1,
                tick,
                timeline: 0,
            },
            [0.0; 3],
            |sink| {
                for slot in 0..10_000_u32 {
                    let x = if slot < 1_500 {
                        tick as f64 * 0.001
                    } else {
                        0.0
                    };
                    sink.push(slot, 1, u128::from(slot), [x, 0.0, 0.0], 1);
                }
            },
        );
        divan::black_box(report);
        divan::black_box(extractor.frame().current());
    });
}
