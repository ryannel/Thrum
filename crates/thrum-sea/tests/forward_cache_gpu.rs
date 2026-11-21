//! The drawn forward sheet and every filtered mip texel, on native and portable Metal.
#[expect(
    dead_code,
    reason = "shared harness includes other independent sea oracles"
)]
mod support;
use std::fmt::Write as _;

#[test]
fn forward_fields_and_full_lean_pyramids_match_the_model() {
    for (portable, wind) in [(false, 8.0), (true, 8.0), (false, 20.0), (true, 20.0)] {
        let spectrum = support::spectrum(wind);
        let choppiness = (0.85
            / thrum_sea_model::contraction_bound(&spectrum.long, 1.0).expect("bound"))
        .min(4.0);
        let Some(mut h) = support::Harness::with_profile(
            portable,
            thrum_sea::Profile {
                choppiness,
                ..Default::default()
            },
        ) else {
            return;
        };
        for time in [10.125, 86400.125] {
            let output = h.frame(&spectrum, time, &[], true).expect("submitted");
            support::check_long(&mut h, &output, &spectrum, time, &[], true);
            for index in 0..2 {
                let texture = h.sea.filtered_texture(index).expect("forward texture");
                let width = texture.width() as usize;
                let layers = texture.depth_or_array_layers() as usize;
                let fields = if index == 0 {
                    &output.cascades[..]
                } else {
                    &output.long[4096 * 12..]
                };
                let base = read(&h.device, &h.queue, texture, 0);
                if index == 1 {
                    let (height, slope) =
                        check_cache(&base, &h.sea.levels, &spectrum, time, choppiness);
                    writeln!(h.record,"filtered cache time {time}: max height {height:.9} m; max rest slope {slope:.9}").expect("record");
                }
                // Every plane, including rest derivatives, rates, J and J^2.
                for layer in 0..layers {
                    for pixel in 0..width * width {
                        let start = ((layer / 5) * width * width + pixel) * 12;
                        let f: [f64; 12] = std::array::from_fn(|c| f64::from(fields[start + c]));
                        let expected = match layer % 5 {
                            0 => [f[0], f[1], f[2], f[3]],
                            1 => [f[4], f[5], f[6], f[7]],
                            2 => [f[8], f[9], f[10], f[11]],
                            3 => [f[4] * f[4], f[4] * f[5], f[5] * f[5], f[11]],
                            _ => {
                                let high = ((layer - 4) * width * width + pixel) * 4;
                                [
                                    f[3] * f[3],
                                    f[0] - base[high],
                                    f[1] - base[high + 1],
                                    f[2] - base[high + 2],
                                ]
                            }
                        };
                        let offset = (layer * width * width + pixel) * 4;
                        for channel in 0..4 {
                            let mut actual = base[offset + channel];
                            let mut budget = 0.001 * expected[channel].abs() + 0.000_001;
                            if layer % 5 == 0 && channel < 3 {
                                actual += base[offset + 4 * width * width * 4 + channel + 1];
                                budget = 0.000_001 * expected[channel].abs() + 0.000_000_1;
                            }
                            let error = (actual - expected[channel]).abs();
                            assert!(error <= budget, "base plane {layer} pixel {pixel}: {error}");
                        }
                    }
                }
                // Direct base-block averages, rather than trusting the parent mip.
                for mip in 1..texture.mip_level_count() {
                    let values = read(&h.device, &h.queue, texture, mip);
                    let block = 1_usize << mip;
                    let child_width = width / block;
                    for layer in 0..layers {
                        for z in 0..child_width {
                            for x in 0..child_width {
                                let mut sum = [0.0; 4];
                                let mut magnitude = [0.0; 4];
                                for dz in 0..block {
                                    for dx in 0..block {
                                        let offset = (layer * width * width
                                            + (z * block + dz) * width
                                            + x * block
                                            + dx)
                                            * 4;
                                        for channel in 0..4 {
                                            let mut value = base[offset + channel];
                                            if layer % 5 == 0 && channel < 3 {
                                                value += base
                                                    [offset + 4 * width * width * 4 + channel + 1];
                                            }
                                            sum[channel] += value;
                                            magnitude[channel] += value.abs();
                                        }
                                    }
                                }
                                let offset =
                                    (layer * child_width * child_width + z * child_width + x) * 4;
                                for channel in 0..4 {
                                    if layer % 5 == 4 && channel > 0 {
                                        continue;
                                    }
                                    let expected = sum[channel] / (block * block) as f64;
                                    // Half storage can round each parent by one ULP.
                                    // Bound accumulated error by the mean absolute
                                    // input, since signed derivatives cancel to zero.
                                    let rounding =
                                        (1.0 + 1.0 / 1024.0_f64).powf(f64::from(mip)) - 1.0;
                                    let mut budget = rounding * magnitude[channel]
                                        / (block * block) as f64
                                        + f64::from(mip) * 0.000_000_1;
                                    let mut actual = values[offset + channel];
                                    if layer % 5 == 0 && channel < 3 {
                                        actual += values[offset
                                            + 4 * child_width * child_width * 4
                                            + channel
                                            + 1];
                                        // The high/low encoding retains about twenty
                                        // significant bits; bound each residual carry
                                        // by parent magnitude, including cancellation.
                                        let paired_rounding =
                                            (1.0 + 0.000_002_5_f64).powf(f64::from(mip)) - 1.0;
                                        budget = paired_rounding * magnitude[channel]
                                            / (block * block) as f64
                                            + f64::from(mip) * 0.000_000_1;
                                    }
                                    assert!(
                                        (actual - expected).abs() <= budget,
                                        "mip {mip} layer {layer} ({x},{z}) channel {channel}: {} vs {expected}",
                                        values[offset + channel]
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        h.record.push_str("All forward-cache texels and all five planes of every texture mip passed. Half-float storage checked against direct base-block means.\n");
        h.save(match (portable, wind > 8.0) {
            (false, false) => "forward-cache-native",
            (true, false) => "forward-cache-portable",
            (false, true) => "forward-cache-high-wind-native",
            (true, true) => "forward-cache-high-wind-portable",
        });
    }
}

fn check_cache(
    base: &[f64],
    levels: &[thrum_sea::CacheLevel; 8],
    spectrum: &thrum_sea_model::Spectrum,
    time: f64,
    choppiness: f64,
) -> (f64, f64) {
    let mut height = 0.0_f64;
    let mut slope = 0.0_f64;
    for (level, info) in levels.iter().enumerate() {
        let model = thrum_sea_model::LongWaves::new(
            spectrum
                .long
                .iter()
                .copied()
                .filter(|c| c.wave_vector[0].hypot(c.wave_vector[1]) <= info.max_wavenumber)
                .collect(),
            choppiness,
        )
        .expect("restricted model");
        for pixel in 0..128 * 128 {
            let rest = [
                info.origin[0] + ((pixel % 128) as f64 + 0.5) * info.texel,
                info.origin[1] + ((pixel / 128) as f64 + 0.5) * info.texel,
            ];
            let cpu = model.forward(rest, time).expect("forward");
            let offset = (level * 5 * 128 * 128 + pixel) * 4;
            let actual = base[offset + 1] + base[offset + 4 * 128 * 128 * 4 + 2];
            height = height.max((actual - cpu.displacement[1]).abs());
            for axis in 0..2 {
                slope = slope.max((base[offset + 128 * 128 * 4 + axis] - cpu.slope[axis]).abs());
            }
        }
    }
    assert!(height < 0.002, "texture height {height}");
    assert!(slope < 0.002, "texture slope {slope}");
    (height, slope)
}

fn half(bits: u16) -> f64 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 31);
    let mantissa = f64::from(bits & 1023);
    assert_ne!(exponent, 31, "nonfinite field moment");
    sign * if exponent == 0 {
        mantissa * 2.0_f64.powi(-24)
    } else {
        (1.0 + mantissa / 1024.0) * 2.0_f64.powi(exponent - 15)
    }
}
fn read(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, mip: u32) -> Vec<f64> {
    thrum_test_support::readback::texture(
        device,
        queue,
        texture,
        thrum_test_support::readback::Mip::level(mip),
        ..,
    )
    .as_chunks::<2>()
    .0
    .iter()
    .map(|b| half(u16::from_le_bytes(*b)))
    .collect()
}
