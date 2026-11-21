//! Paired CPU upload measurement on the same Metal fixture.
use crate::support;

#[test]
fn cpu_upload_cost() {
    use std::fmt::Write as _;
    let Some(mut h) = support::Harness::new(false) else {
        return;
    };
    let spectrum = support::spectrum(5.0);
    let points = support::points();
    let mut samples = Vec::new();
    for frame in 0..48 {
        let mut encoder = h
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let start = std::time::Instant::now();
        h.sea
            .upload(
                &h.queue,
                &mut encoder,
                &spectrum,
                f64::from(frame),
                support::ANCHOR,
                &points,
            )
            .expect("valid upload");
        let micros = start.elapsed().as_secs_f64() * 1e6;
        h.queue.submit([encoder.finish()]);
        h.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("upload complete");
        if frame >= 8 {
            samples.push(micros);
        }
    }
    samples.sort_by(f64::total_cmp);
    writeln!(
        h.record,
        "CPU upload only, 8 warmups, 40 samples, 4096 queries: median_us={} p95_us={} max_us={}",
        samples[20], samples[38], samples[39]
    )
    .expect("record");
    h.save("cpu-upload");
}
