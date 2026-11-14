//! Six-hour birth-count statistics, and independent generation centres.
pub mod support;
use std::f64::consts::TAU;
use support::field;

#[test]
fn births_have_no_generation_beat() {
    for seed in [1, 7, 42] {
        let field = field(seed, 0.3);
        let mut counts = [0.0_f64; 360];
        let mut dx = Vec::new();
        let mut products = 0.0;
        let mut sum = 0.0;
        let mut squares = 0.0;
        let mut n = 0.0;
        for i in 0..30 {
            for j in 0..30 {
                for g in -2..16 {
                    let cell = field.cell(i, j, g);
                    if cell.threshold < 0.4 && cell.birth >= 0.0 && cell.birth < 21_600.0 {
                        #[expect(
                            clippy::cast_possible_truncation,
                            clippy::cast_sign_loss,
                            reason = "bounded minute index"
                        )]
                        let bin = (cell.birth / 60.0) as usize;
                        counts[bin] += 1.0;
                    }
                    let next = field.cell(i, j, g + 1);
                    let a = cell.centre[0] / 1300.0 - i as f64 - 0.5;
                    let b = next.centre[0] / 1300.0 - i as f64 - 0.5;
                    products += a * b;
                    sum += a;
                    squares += a * a;
                    n += 1.0;
                    dx.push((cell.centre[0] - next.centre[0]).abs());
                }
            }
        }
        let mean = counts.iter().sum::<f64>() / 360.0;
        let variance = counts.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / 360.0;
        assert!(
            (0.75..=1.25).contains(&(variance / mean)),
            "seed {seed}, variance/mean {}",
            variance / mean
        );
        let spectrum: Vec<f64> = (1..=180)
            .map(|k| {
                let (mut re, mut im) = (0.0, 0.0);
                for (index, value) in counts.iter().enumerate() {
                    let angle = TAU * f64::from(k) * index as f64 / 360.0;
                    re += (value - mean) * angle.cos();
                    im += (value - mean) * angle.sin();
                }
                re * re + im * im
            })
            .collect();
        let average = spectrum.iter().sum::<f64>() / 180.0;
        let mut peaks = Vec::new();
        for harmonic in 1_usize..=4 {
            // Evaluate the actual generation frequency, including half-bin
            // frequencies, rather than hiding a beat between FFT bins.
            let mut re = 0.0;
            let mut im = 0.0;
            for (minute, count) in counts.iter().enumerate() {
                let angle = TAU * harmonic as f64 * (minute as f64 + 0.5) * 60.0 / 1600.0;
                re += (count - mean) * angle.cos();
                im += (count - mean) * angle.sin();
            }
            let power = re * re + im * im;
            peaks.push(power / average);
            assert!(
                power < 4.6 * average,
                "seed {seed}, harmonic {harmonic}: {}",
                power / average
            );
        }
        assert!((products / n - (sum / n).powi(2)).abs() / (squares / n) < 0.04);
        assert!(dx.iter().sum::<f64>() / dx.len() as f64 > 350.0);
        println!(
            "seed={seed}, mean={mean:.6}, variance/mean={:.6}, harmonic_power/mean={peaks:?}",
            variance / mean
        );
    }
}
