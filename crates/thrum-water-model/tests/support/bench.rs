//! CPU preparation and highest-column costs. No graphics device is requested.
mod sheets;
mod surf_map;
use std::hint::black_box;
use thrum_water_model::{
    Bed, BedPyramid, CutConfig, ShoreConfig, ShoreTransform, Source, Stage, cut,
};

fn main() {
    divan::main();
}

fn bed() -> Bed {
    Bed::new(
        [-500.0; 2],
        4.0,
        [251; 2],
        (0..251)
            .flat_map(|z| {
                (0..251).map(move |x| {
                    let (x, z) = (f64::from(x) * 4.0 - 500.0, f64::from(z) * 4.0 - 500.0);
                    -40.0 + 46.0 * (-(x * x + z * z) / (2.0 * 150.0_f64.powi(2))).exp()
                })
            })
            .collect(),
    )
    .expect("finite analytic bed")
}

#[divan::bench(sample_count = 3, sample_size = 1)]
#[expect(
    clippy::print_stdout,
    reason = "benchmark records retained sample payload bytes"
)]
fn preparation(bencher: divan::Bencher<'_, '_>) {
    let bed = bed();
    let config = ShoreConfig {
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 2.0,
            value: 0.0,
        },
        margin: 1.5,
        floor: 0.5,
        fetch: 1400.0,
        seed: None,
        source: Source::Water,
    };
    let pyramid = BedPyramid::new(&bed);
    println!(
        "grid=251x251, spacing=4m, stage=0..2m, pyramid_bytes={}",
        pyramid.bytes()
    );
    bencher.bench_local(|| {
        black_box(ShoreTransform::prepare(bed.clone(), config).expect("validated preparation"))
    });
}

#[divan::bench(args = [0, 1, 2], sample_count = 50, sample_size = 1)]
#[expect(
    clippy::print_stdout,
    reason = "benchmark records the actual column node count"
)]
fn column(bencher: divan::Bencher<'_, '_>, sheet: usize) {
    let point = if sheet == 2 {
        [-29.5, 12.0]
    } else {
        [3.8, 0.0]
    };
    let sheet = sheets::Trains::new(sheet);
    let first = cut(&sheet, point, CutConfig::default()).expect("covered column");
    println!(
        "point={point:?}, cut_nodes={}, hits={}",
        first.nodes, first.hits
    );
    bencher.bench_local(|| {
        black_box(cut(&sheet, black_box(point), CutConfig::default()).expect("covered column"))
    });
}

#[divan::bench(args = [0, 1, 2], sample_count = 50, sample_size = 1)]
#[expect(
    clippy::print_stdout,
    reason = "benchmark records production column nodes and hits"
)]
fn production_column(bencher: divan::Bencher<'_, '_>, case: usize) {
    let (map, field) = surf_map::fixture(case);
    let sheet = map
        .surf_sheet(&field, 0, 0.0)
        .expect("prepared surf capability");
    let point = [[-6.67, -9.62], [-7.21, -8.20], [-24.34, -7.10]][case];
    let first = cut(&sheet, point, CutConfig::default()).expect("covered production column");
    println!(
        "point={point:?}, cut_nodes={}, hits={}",
        first.nodes, first.hits
    );
    bencher.bench_local(|| {
        black_box(
            cut(&sheet, black_box(point), CutConfig::default()).expect("covered production column"),
        )
    });
}
