//! Local closed-record clone cost fixture. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_record_cost`

use std::{hint::black_box, time::Instant};
use thrum_core::{
    ClosedPublicationValue, ClosedValueLimits, PublicationBytes, PublicationPool, PublicationRecord,
};

const BATCH: usize = 4_096;
const WARMUP: usize = 50;
const SAMPLES: usize = 250;

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percent / 100]
}

fn measure<const BLOBS: usize>(label: &str, value: &PublicationRecord<8, BLOBS>) {
    let mut copies = Vec::with_capacity(BATCH);
    let mut samples = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let start = Instant::now();
        for _ in 0..BATCH {
            copies.push(black_box(value).clone());
        }
        black_box(&copies);
        copies.clear();
        if sample >= WARMUP {
            samples.push(start.elapsed().as_nanos());
        }
    }
    println!(
        "record/{label}: clones_and_drops_per_batch={BATCH} batch_ns[p50/p95/p99]={}/{}/{}",
        percentile(&mut samples, 50),
        percentile(&mut samples, 95),
        percentile(&mut samples, 99),
    );
}

fn measure_check<const BLOBS: usize>(
    label: &str,
    value: &PublicationRecord<8, BLOBS>,
    pool: &PublicationPool,
) {
    let limits = ClosedValueLimits {
        max_inline_bytes: 8,
        max_payload_slots: 1,
        max_payload_bytes: 512,
    };
    let mut samples = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let start = Instant::now();
        for _ in 0..BATCH {
            black_box(value)
                .validate_for_pool(black_box(pool), limits)
                .unwrap();
        }
        if sample >= WARMUP {
            samples.push(start.elapsed().as_nanos());
        }
    }
    println!(
        "record/{label}: validations_per_batch={BATCH} batch_ns[p50/p95/p99]={}/{}/{}",
        percentile(&mut samples, 50),
        percentile(&mut samples, 95),
        percentile(&mut samples, 99),
    );
}

fn main() {
    let pool = PublicationPool::new(usize::MAX);
    let inline = 7_u64.to_le_bytes();
    let fixed = PublicationRecord::<8, 0>::try_new(&pool, inline, []).unwrap();
    let payload = PublicationBytes::try_copy_from_slice(&pool, &[9; 512]).unwrap();
    let with_payload = PublicationRecord::<8, 1>::try_new(&pool, inline, [Some(payload)]).unwrap();
    println!(
        "I1 record fixture host={}/{} profile=release",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    measure("fixed", &fixed);
    measure("one_shared_payload", &with_payload);
    measure_check("fixed_check", &fixed, &pool);
    measure_check("one_shared_payload_check", &with_payload, &pool);
}
