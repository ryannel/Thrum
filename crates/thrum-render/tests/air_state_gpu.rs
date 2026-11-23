//! Read the prepared Metal products on the first submission after each change.
use thrum_render::{
    atmosphere::{Aerosol, AtmosphereFrame, EnvironmentFrame},
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene},
};
use thrum_test_support::readback::{Mip, Readback};

fn submit(
    prepared: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
    frame: AtmosphereFrame,
) -> (EnvironmentFrame, [Vec<u8>; 4], [u32; 2]) {
    prepared
        .update_frame(queue, target, scene, FrameLighting::Atmosphere(frame), 1.0)
        .expect("air update");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, caps.generation(), target, None)
        .expect("encode charts");
    let (sample, charts) = prepared.atmosphere_witness().expect("prepared atmosphere");
    let extent = [charts[0].width(), charts[0].height() / 3];
    let reads =
        charts.map(|texture| Readback::texture(device, &mut encoder, texture, Mip::default(), ..));
    let submission = queue.submit([encoder.finish()]);
    prepared.commit_frame();
    (
        sample,
        reads.map(|read| read.wait(device, submission.clone())),
        extent,
    )
}

fn band(chart: &[u8], extent: [u32; 2], texel: usize, band: usize) -> f32 {
    let plane = (extent[0] * extent[1]) as usize;
    let offset = ((plane * (1 + band / 4) + texel) * 4 + band % 4) * 2;
    half::f16::from_le_bytes(chart[offset..offset + 2].try_into().expect("f16")).to_f32()
}

fn spectral_amount_and_kind(
    clear: &[u8],
    grey: &[u8],
    coloured: &[u8],
    extent: [u32; 2],
    amount: f32,
) {
    let wavelengths: [f32; 8] = [
        428.84, 450.21, 471.18, 514.23, 550.21, 574.75, 597.70, 630.96,
    ];
    let max_error = 0.003;
    let mut cases = 0;
    for texel in (0..(extent[0] * extent[1]) as usize).step_by(16) {
        // Infer the aerosol column from the two grey runs. The unchanged gas
        // columns cancel. Predict the coloured result band by band.
        let clear_550 = band(clear, extent, texel, 4);
        let grey_550 = band(grey, extent, texel, 4);
        if !(0.10..=0.70).contains(&grey_550) || clear_550 - grey_550 < 0.15 {
            continue;
        }
        let column = (clear_550 / grey_550).ln() / (amount - 4.4e-6);
        for (index, wavelength) in wavelengths.into_iter().enumerate() {
            let grey = band(grey, extent, texel, index);
            let coloured = band(coloured, extent, texel, index);
            let delta = amount * ((wavelength / 550.0).powf(-1.3) - 1.0);
            let expected = grey * (-column * delta).exp();
            assert!(
                (coloured - expected).abs() < max_error,
                "spectral texel {texel}, {wavelength} nm: {coloured} vs {expected}"
            );
            if index == 0 && grey > 0.08 {
                assert!(
                    (grey - expected).abs() > max_error,
                    "flat aerosol control must fail"
                );
                cases += 1;
            }
        }
    }
    assert!(cases > 10, "{cases} resolved blue-band falsifiers");
}

#[test]
fn metal_aerosol_defaults_and_live_charts() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "air profile charts",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    assert_eq!(gpu.adapter.get_info().backend, wgpu::Backend::Metal);
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let scene = SyntheticScene::grid(1).expect("small world");
    let base = AtmosphereFrame::new([0.98, 0.17, 0.0], 127_000.0, [1.0; 3], 2.0, 1)
        .expect("daylight")
        .with_nightglow();
    let default = base.with_aerosol(Aerosol::default());
    let make = |frame| {
        pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
            &caps,
            &queue,
            &scene,
            FrameLighting::Atmosphere(frame),
            1.0,
        ))
        .expect("prepare air")
    };
    let mut fixed = make(base);
    let mut fixed_target = OffscreenTarget::new(&device, &fixed, 16, 16).expect("fixed target");
    let (fixed_sun, fixed_charts, _) = submit(
        &mut fixed,
        &mut fixed_target,
        &scene,
        &caps,
        &queue,
        &device,
        base,
    );
    let mut live = make(default);
    let mut target = OffscreenTarget::new(&device, &live, 16, 16).expect("live target");
    let (clear, clear_charts, extent) = submit(
        &mut live,
        &mut target,
        &scene,
        &caps,
        &queue,
        &device,
        default,
    );
    for (index, (before, after)) in fixed_charts.iter().zip(&clear_charts).enumerate() {
        assert!(before == after, "default chart {index} changed bytes");
    }
    assert_eq!(clear.surface_sun, fixed_sun.surface_sun);

    let amount = 3.912_023 / 20_000.0 - 11.6e-6;
    let grey = base.with_aerosol(Aerosol::new(amount, 0.0).expect("grey haze"));
    let fine = base.with_aerosol(Aerosol::new(amount, 1.3).expect("coloured haze"));
    let (thick, thick_charts, _) =
        submit(&mut live, &mut target, &scene, &caps, &queue, &device, grey);
    let (coloured, coloured_charts, _) =
        submit(&mut live, &mut target, &scene, &caps, &queue, &device, fine);
    assert_eq!(Aerosol::CHART_LAG_FRAMES, 1);
    assert_eq!(clear.epoch, thick.epoch);
    assert_eq!(clear.epoch, coloured.epoch);
    assert_ne!(clear.surface_sun, thick.surface_sun);
    assert_ne!(thick.surface_sun, coloured.surface_sun);
    for (old, new) in clear_charts.iter().zip(&thick_charts) {
        assert!(
            old != new,
            "amount must reach each chart in the first new frame"
        );
    }
    for (old, new) in thick_charts.iter().zip(&coloured_charts) {
        assert!(
            old != new,
            "kind must reach each chart in the first new frame"
        );
    }
    let night = |texel: &[u8]| [texel[6], texel[7]];
    assert!(
        clear_charts[2]
            .as_chunks::<8>()
            .0
            .iter()
            .zip(thick_charts[2].as_chunks::<8>().0)
            .any(|(old, new)| night(old) != night(new)),
        "sky night transport must respond to haze"
    );
    spectral_amount_and_kind(
        &clear_charts[0],
        &thick_charts[0],
        &coloured_charts[0],
        extent,
        amount,
    );
    let (restored, restored_charts, _) = submit(
        &mut live,
        &mut target,
        &scene,
        &caps,
        &queue,
        &device,
        default,
    );
    assert_eq!(clear.epoch, restored.epoch);
    assert!(
        fixed_charts == restored_charts,
        "restoring defaults restores all chart bytes"
    );
}
