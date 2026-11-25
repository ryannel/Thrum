//! Qualified paint above the moving, reconstructed authored V1 valley.

#[cfg(target_os = "macos")]
mod support {
    pub(crate) mod interface_valley_fixture;
}
#[cfg(target_os = "macos")]
use support::interface_valley_fixture as fixture;

#[cfg(target_os = "macos")]
use std::{collections::BTreeSet, time::Duration};
#[cfg(target_os = "macos")]
use thrum_render::{
    present::SurfaceProfile,
    runtime::{SubmittedImage, SurfaceOutput},
    synthetic::RenderScale,
};
#[cfg(target_os = "macos")]
use v1_game::GameHost;

thrum_test_support::macos_gpu_test! {
    fn overlay_is_after_reconstruction_and_display_and_leaves_uncovered_valley_bits_intact() {
        let Some(gpu) = thrum_test_support::open(
            "reconstructed interface valley",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Native,
            &thrum_test_support::Restriction::default(),
        ) else { return };
        let scratch = thrum_test_support::Scratch::new("interface-valley");
        let store = fixture::cook(scratch.path());
        for recipe in ["portable", "apple-tile"] {
            let caps = gpu.capabilities.clone().with_recipe_override(recipe).expect("selected Metal recipe");
            for format in [wgpu::TextureFormat::Bgra8Unorm, wgpu::TextureFormat::Rgba16Float] {
                let extent = [136, 86];
                let profile = SurfaceProfile { format, extent };
                let float = format == wgpu::TextureFormat::Rgba16Float;
                let mut plain = fixture::host(&gpu.device, &caps, &gpu.queue, profile, &store, false);
                let mut painted = fixture::host(&gpu.device, &caps, &gpu.queue, profile, &store, true);
                let view = fixture::drawable(&gpu.device, profile);
                let surface = SurfaceOutput { view: &view, extent };
                let mut jitter = BTreeSet::new();
                let mut exposures = BTreeSet::new();
                let mut highlight = false;
                let mut centre = None;
                let mut moved = false;
                for index in 0u32..12 {
                    let time = Duration::from_millis(u64::from(index) * 17);
                    let yaw = -std::f32::consts::FRAC_PI_2 - 0.046 + index as f32 * 0.006;
                    for host in [&mut plain, &mut painted] {
                        host.set_camera_yaw(yaw).expect("move continuously across the valley");
                        if matches!(index, 0 | 6) {
                            let hours = if index == 0 { 17.9 } else { 23.0 };
                            host.set_time_of_day(hours / 24.0).expect("change scene exposure");
                        }
                    }
                    let (a_sample, a) = fixture::frame(&mut plain, &surface, time);
                    let (b_sample, b) = fixture::frame(&mut painted, &surface, time);
                    assert_eq!(a.source, b.source);
                    assert_eq!(a_sample.jitter_phase, b_sample.jitter_phase);
                    assert_eq!(a_sample.render_extent, RenderScale::Half.extent(extent));
                    assert_eq!(b_sample.render_extent, a_sample.render_extent);
                    jitter.insert(b_sample.jitter_phase.expect("temporal jitter ran"));
                    if let Some(exposure) = b.exposure_gpu_words {
                        exposures.insert(exposure[1]);
                    }
                    highlight |= compare(&a, &b, float);
                    let bytes = if float { 8 } else { 4 };
                    let valley = a.rgba[(135 + 40 * extent[0] as usize) * bytes..][..bytes].to_vec();
                    moved |= centre.as_ref().is_some_and(|previous| previous != &valley);
                    centre = Some(valley);
                    plain.recycle_submitted_image(a);
                    painted.recycle_submitted_image(b);
                }
                assert!(jitter.len() >= 4, "reconstruction must run over multiple phases");
                assert!(exposures.len() >= 2, "valley exposure must change");
                assert!(moved, "the uncovered valley changes under the steady card");
                assert!(!float || highlight, "the uncovered float valley must retain a scene highlight");
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn compare(plain: &SubmittedImage, painted: &SubmittedImage, float: bool) -> bool {
    let bytes = if float { 8 } else { 4 };
    let [width, height] = plain.extent.map(|axis| axis as usize);
    assert_eq!(plain.extent, painted.extent);
    assert_eq!(plain.pixel_source, painted.pixel_source);
    assert_eq!(plain.rgba.len(), width * height * bytes);
    assert_eq!(plain.rgba.len(), painted.rgba.len());
    let mut highlight = false;
    let encode = |linear: f32| {
        let code = if linear <= 0.003_130_8 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        (code * 255.0).round()
    };
    for (index, (a, b)) in plain
        .rgba
        .chunks_exact(bytes)
        .zip(painted.rgba.chunks_exact(bytes))
        .enumerate()
    {
        let (x, y) = (index % width, index / width);
        let panel = (8..120).contains(&x) && (8..66).contains(&y);
        let anchor = (width - 10..width - 6).contains(&x) && (height - 10..height - 6).contains(&y);
        if !panel && !anchor {
            assert_eq!(a, b, "uncovered pixel at {x},{y}");
            highlight |= float
                && a[..6].as_chunks::<2>().0.iter().any(|channel| {
                    half::f16::from_le_bytes([channel[0], channel[1]]).to_f32() > 1.0
                });
        }
        let corner = (8..17).contains(&x) && (8..12).contains(&y);
        if corner || anchor {
            let texel = if anchor {
                [223, 229, 255]
            } else {
                [255, 134, 61]
            };
            for (channel, code) in texel.into_iter().enumerate() {
                if float {
                    let value =
                        half::f16::from_le_bytes([b[channel * 2], b[channel * 2 + 1]]).to_f32();
                    assert_eq!(
                        encode(value),
                        f32::from(code),
                        "float opaque pixel at {x},{y}"
                    );
                } else {
                    assert_eq!(b[channel], code, "SDR opaque pixel at {x},{y}");
                }
            }
        }
    }
    highlight
}
