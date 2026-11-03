//! Exact source admission for render-v1 local-light unit luminance.
//!
//! This deliberately does not change the constant-light-v1 environment predicate.

use std::cmp::Ordering;

const SCALED_WEIGHTS: [u64; 3] = [212_600_000, 715_200_000, 72_200_000];
const LOWER: u64 = 999_999_999;
const UPPER: u64 = 1_000_000_001;
const ERROR: f64 = 1.0 / 65_536.0;

/// Test exact decoded-f64 RGB against rational Rec.709 weights and decimal 1e-9.
///
/// All arithmetic in the defining predicate is real arithmetic: the weights are
/// 1063/5000, 3576/5000 and 361/5000; the tolerance is 1/1_000_000_000.
/// Invalid finite/nonnegative source quantities are refused here as well.
pub(crate) fn has_unit_luminance(rgb: [f64; 3]) -> bool {
    if rgb.iter().any(|x| !x.is_finite() || *x < 0.0 || *x > 16.0) {
        // Even the smallest weight times 16 is 1.1552, above the upper bound.
        return false;
    }
    if let Some(answer) = certified_interval(rgb) {
        return answer;
    }
    exact_comparison(rgb)
}

fn certified_interval([r, g, b]: [f64; 3]) -> Option<bool> {
    // Multiplying the defining inequality by 1e9 gives integer endpoints:
    // 999999999 <= 200000*(1063*r + 3576*g + 361*b) <= 1000000001.
    // Every coefficient/endpoint is exactly representable. With each input in
    // [0,16], each of the three products is below 2^16 (rounding error <=2^-38),
    // and the two sums are below 2^17 (error <=2^-37). Scaling amplifies these
    // errors by 200000; its own rounding contributes <=2^-20 because the exact
    // result is below 2^34. Thus total error <=200000*7*2^-38 + 2^-20
    // <2^-16. These absolute bounds cover subnormal rounding too. The four
    // endpoint +/- ERROR values below are exactly representable.
    let scaled = ((1063.0 * r + 3576.0 * g) + 361.0 * b) * 200_000.0;
    if scaled >= LOWER as f64 + ERROR && scaled <= UPPER as f64 - ERROR {
        Some(true)
    } else if scaled < LOWER as f64 - ERROR || scaled > UPPER as f64 + ERROR {
        Some(false)
    } else {
        None
    }
}

fn exact_comparison(rgb: [f64; 3]) -> bool {
    // A decoded nonnegative f64 is an integer multiple of 2^-1074. The largest
    // permitted channel is 16, so the scaled sum is <=16e9: 1108 bits suffice
    // after that common scaling. Eighteen limbs provide 1152 bits, on the stack.
    let mut sum = [0_u64; 18];
    for (value, weight) in rgb.into_iter().zip(SCALED_WEIGHTS) {
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as usize;
        let fraction = bits & ((1_u64 << 52) - 1);
        let (significand, shift) = if exponent == 0 {
            (fraction, 0)
        } else {
            (fraction | (1_u64 << 52), exponent - 1)
        };
        add_shifted(
            &mut sum,
            u128::from(significand) * u128::from(weight),
            shift,
        );
    }
    compare_bound(&sum, LOWER) != Ordering::Less && compare_bound(&sum, UPPER) != Ordering::Greater
}

fn add_shifted(sum: &mut [u64; 18], value: u128, shift: usize) {
    let word = shift / 64;
    let offset = shift % 64;
    let low = value as u64;
    let high = (value >> 64) as u64;
    if offset == 0 {
        add_word(sum, word, low);
        add_word(sum, word + 1, high);
    } else {
        add_word(sum, word, low << offset);
        add_word(sum, word + 1, (high << offset) | (low >> (64 - offset)));
        add_word(sum, word + 2, high >> (64 - offset));
    }
}

fn add_word(sum: &mut [u64; 18], mut word: usize, mut value: u64) {
    while value != 0 {
        let (result, carry) = sum[word].overflowing_add(value);
        sum[word] = result;
        value = u64::from(carry);
        word += 1;
    }
}

fn compare_bound(sum: &[u64; 18], bound: u64) -> Ordering {
    // The integer endpoint shifted left by 1074 occupies only limbs 16 and 17.
    sum[17]
        .cmp(&(bound >> 14))
        .then_with(|| sum[16].cmp(&(bound << 50)))
        .then_with(|| sum[..16].iter().any(|word| *word != 0).cmp(&false))
}

#[cfg(test)]
mod tests {
    use super::{SCALED_WEIGHTS, certified_interval, exact_comparison, has_unit_luminance};

    #[test]
    fn common_colours_and_invalid_quantities() {
        for rgb in [[1.0; 3], [0.0, 1.0, 3.9445983379501386]] {
            assert!(has_unit_luminance(rgb));
            assert_eq!(certified_interval(rgb), Some(true));
        }
        for rgb in [
            [0.0; 3],
            [16.0; 3],
            [f64::MAX, 0.0, 0.0],
            [f64::INFINITY, 0.0, 0.0],
            [f64::NAN, 0.0, 0.0],
            [-f64::from_bits(1), 1.0, 1.0],
            [f64::from_bits(1); 3],
        ] {
            assert!(!has_unit_luminance(rgb), "{rgb:?}");
        }
        assert!(has_unit_luminance([-0.0, 1.0, 3.9445983379501386]));
    }

    #[test]
    fn decimal_weight_rounding_disagreements() {
        let cases = [
            ([4.703668857008466, 0.0, 0.0], false),
            ([0.0, 0.0, 13.850415526315789], true),
        ];
        for (rgb, expected) in cases {
            let rounded = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
            assert_ne!((rounded - 1.0_f64).abs() <= 1e-9, expected);
            assert_eq!(certified_interval(rgb), None);
            assert_eq!(has_unit_luminance(rgb), expected);
        }
    }

    #[test]
    fn python_fraction_oracle_at_both_endpoints() {
        // Independently generated using Python Fraction.from_float, integer
        // weights /5000, tolerance Fraction(1,10**9), and nextafter either way
        // from float((1 +/- tolerance)*5000/weight). Mixed cases solve for blue
        // from exact Fraction.from_float red/green before nextafter sampling.
        let cases = [
            ([0x4012d08e91a0429f, 0, 0], false),
            ([0x4012d08e91a042a0, 0, 0], false),
            ([0x4012d08e91a042a1, 0, 0], true),
            ([0x4012d08e9241e088, 0, 0], true),
            ([0x4012d08e9241e089, 0, 0], false),
            ([0x4012d08e9241e08a, 0, 0], false),
            ([0, 0x3ff65f11c0a07d09, 0], false),
            ([0, 0x3ff65f11c0a07d0a, 0], true),
            ([0, 0x3ff65f11c0a07d0b, 0], true),
            ([0, 0x3ff65f11c160a82f, 0], true),
            ([0, 0x3ff65f11c160a830, 0], true),
            ([0, 0x3ff65f11c160a831, 0], false),
            ([0, 0, 0x402bb369a9052024], false),
            ([0, 0, 0x402bb369a9052025], true),
            ([0, 0, 0x402bb369a9052026], true),
            ([0, 0, 0x402bb369a9f312e9], true),
            ([0, 0, 0x402bb369a9f312ea], true),
            ([0, 0, 0x402bb369a9f312eb], false),
            (
                [0x3fd0000000000000, 0x3fe8000000000000, 0x4016bd573882ce1c],
                false,
            ),
            (
                [0x3fd0000000000000, 0x3fe8000000000000, 0x4016bd573882ce1d],
                false,
            ),
            (
                [0x3fd0000000000000, 0x3fe8000000000000, 0x4016bd573882ce1e],
                true,
            ),
            (
                [0x3fd0000000000000, 0x3fe8000000000000, 0x4016bd573a5eb3a7],
                true,
            ),
            (
                [0x3fd0000000000000, 0x3fe8000000000000, 0x4016bd573a5eb3a8],
                true,
            ),
            (
                [0x3fd0000000000000, 0x3fe8000000000000, 0x4016bd573a5eb3a9],
                false,
            ),
            (
                [0x3fb999999999999a, 0x3ff3333333333333, 0x3ffab41f300ca38b],
                false,
            ),
            (
                [0x3fb999999999999a, 0x3ff3333333333333, 0x3ffab41f300ca38c],
                false,
            ),
            (
                [0x3fb999999999999a, 0x3ff3333333333333, 0x3ffab41f300ca38d],
                true,
            ),
            (
                [0x3fb999999999999a, 0x3ff3333333333333, 0x3ffab41f377c39b7],
                true,
            ),
            (
                [0x3fb999999999999a, 0x3ff3333333333333, 0x3ffab41f377c39b8],
                true,
            ),
            (
                [0x3fb999999999999a, 0x3ff3333333333333, 0x3ffab41f377c39b9],
                false,
            ),
        ];
        for (bits, expected) in cases {
            let rgb = bits.map(f64::from_bits);
            assert_eq!(certified_interval(rgb), None, "{bits:x?}");
            assert_eq!(has_unit_luminance(rgb), expected, "{bits:x?}");
            assert_eq!(exact_comparison(rgb), expected, "{bits:x?}");
        }
    }

    #[test]
    fn fixed_denominator_integer_oracle() {
        // This oracle uses one u128 numerator over 2^48, independently of f64
        // decoding or shifted limbs. All input conversions are exact.
        const DENOMINATOR: u64 = 1 << 48;
        let mut state = 0x0123_4567_89ab_cdef_u64;
        for sample in 0..30_000 {
            let numerators = std::array::from_fn::<_, 3, _>(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state % (16 * DENOMINATOR + 1)
            });
            let mut rgb = numerators.map(|n| n as f64 / DENOMINATOR as f64);
            if sample % 3 != 0 {
                // Move blue close to one tolerance boundary for meaningful
                // acceptance/fallback coverage, retaining the common grid.
                let target = if sample % 2 == 0 {
                    999_999_999.0
                } else {
                    1_000_000_001.0
                };
                let blue =
                    (target - rgb[0] * 212_600_000.0 - rgb[1] * 715_200_000.0) / 72_200_000.0;
                if (0.0..=16.0).contains(&blue) {
                    rgb[2] = (blue * DENOMINATOR as f64).round() / DENOMINATOR as f64;
                }
            }
            let numerator: u128 = rgb
                .into_iter()
                .zip(SCALED_WEIGHTS)
                .map(|(x, weight)| (x * DENOMINATOR as f64) as u128 * u128::from(weight))
                .sum();
            let expected = (999_999_999_u128 * u128::from(DENOMINATOR)
                ..=1_000_000_001_u128 * u128::from(DENOMINATOR))
                .contains(&numerator);
            assert_eq!(has_unit_luminance(rgb), expected, "{rgb:?}");
            assert_eq!(exact_comparison(rgb), expected, "{rgb:?}");
        }
    }

    #[test]
    fn subnormal_and_full_exponent_span_do_not_disappear() {
        for bits in [1, (1_u64 << 52) - 1, 1_u64 << 52] {
            let tiny = f64::from_bits(bits);
            for base in [4.703668857008466, f64::from_bits(0x4012d08e91a042a1)] {
                let rgb = [base, tiny, tiny];
                // The endpoint gap is many orders larger than these tiny
                // additions; Python Fraction yields the same answers.
                let expected = base.to_bits() == 0x4012d08e91a042a1;
                assert_eq!(has_unit_luminance(rgb), expected);
                assert_eq!(exact_comparison(rgb), expected);
            }
        }
    }

    #[test]
    fn tiny_positive_channels_can_determine_admission() {
        // Python Fraction oracle: choose red just below each exact endpoint,
        // green just below the remaining exact weighted difference, then sample
        // blue and its neighbours. A one-ULP change around blue ~= 1e-32 decides
        // admission even though adding either small channel in f64 loses it.
        let cases = [
            (
                [0x4012d08e91a042a0, 0x3c939996b0f263ad, 0x3977d36885a0533c],
                false,
            ),
            (
                [0x4012d08e91a042a0, 0x3c939996b0f263ad, 0x3977d36885a0533d],
                false,
            ),
            (
                [0x4012d08e91a042a0, 0x3c939996b0f263ad, 0x3977d36885a0533e],
                true,
            ),
            (
                [0x4012d08e9241e088, 0x3cb194298101d3e5, 0x39649a6ca6bdee70],
                true,
            ),
            (
                [0x4012d08e9241e088, 0x3cb194298101d3e5, 0x39649a6ca6bdee71],
                false,
            ),
            (
                [0x4012d08e9241e088, 0x3cb194298101d3e5, 0x39649a6ca6bdee72],
                false,
            ),
        ];
        for (bits, expected) in cases {
            let rgb = bits.map(f64::from_bits);
            assert_eq!(certified_interval(rgb), None);
            assert_eq!(has_unit_luminance(rgb), expected, "{bits:x?}");
        }
    }
}
