//! Metal's submitted scene-linear light through the water to a matte panel.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
use support::optics::{EXTENT, Panel, Rig, attach, camera, sun, water_with_profile};

#[cfg(target_os = "macos")]
fn panel(
    gpu: &thrum_test_support::Gpu,
    distance: f32,
    scale: f32,
    radiant: bool,
) -> (f32, [f32; 3]) {
    let depth = -2.0;
    let panel = Panel {
        centre: [0.0, depth, distance + 0.1],
        half_size: [1.0, 0.8, 0.1],
        radiant,
    };
    let view = camera([0.0, depth, 0.0], [0.0, depth, 1.0], 0.8, 1.0 / 32768.0)
        .with_storage_scale_probe(Some(scale))
        .expect("fixed storage multiplier");
    let mut rig = Rig::new(gpu, &view, sun(0.0, 0.0), &[panel], None);
    attach(&mut rig, water_with_profile(None, true, true));
    let (scale, pixels) = rig.scene_colour();
    let center = ((EXTENT[1] / 2) * EXTENT[0] + EXTENT[0] / 2) as usize;
    (scale, pixels[center])
}

#[cfg(target_os = "macos")]
fn contribution(gpu: &thrum_test_support::Gpu, distance: f32, scale: f32) -> (f32, [f32; 3]) {
    let (off_scale, matte) = panel(gpu, distance, scale, false);
    let (on_scale, witness) = panel(gpu, distance, scale, true);
    assert_eq!(off_scale.to_bits(), on_scale.to_bits());
    assert_eq!(
        on_scale.to_bits(),
        scale.to_bits(),
        "GPU storage multiplier"
    );
    (
        on_scale,
        std::array::from_fn(|i| (witness[i] - matte[i]) / on_scale),
    )
}

thrum_test_support::macos_gpu_test! {
fn the_lens_applies_one_water_path_and_one_exposure() {
    let Some(gpu) = thrum_test_support::gpu("underwater optical handoff", Some(wgpu::Backend::Metal)) else { return };
    let low = 1.0 / 1024.0;
    let high = low * 16.0;
    let near = 2.0;
    let far = 4.0;
    let (low_word, reference) = contribution(&gpu, near, low);
    let (_, distant) = contribution(&gpu, far, low);
    let (high_word, scaled) = contribution(&gpu, near, high);
    assert_eq!((high_word / low_word).to_bits(), 16.0_f32.to_bits());
    eprintln!("Submitted GPU storage multipliers {low_word} and {high_word}");
    for (index, extinction) in thrum_water::Optics::default().extinction.into_iter().enumerate() {
        let expected = (-extinction * (far - near)).exp();
        let ratio = distant[index] / reference[index];
        let exposure = scaled[index] / reference[index];
        eprintln!("Metal channel {index}: measured transmission {ratio:.5}, once {expected:.5}, twice {:.5}, exposure ratio {exposure:.5}", expected * expected);
        let tolerance = 0.03;
        assert!(reference[index] > 100.0);
        assert!(expected - expected * expected > 2.0 * tolerance);
        assert!((ratio - expected).abs() < tolerance);
        assert!((exposure - 1.0).abs() < tolerance);
    }
}
}
