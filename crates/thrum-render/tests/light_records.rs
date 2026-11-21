//! Exact Metal output from the scene renderer in each lighting mode.
mod support;

use glam::Vec3;
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, PhotometricEnvironment, SyntheticLight, SyntheticScene},
};

#[test]
fn light_modes_match_the_pre_split_pixels() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "light record pixels",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let ambient_only = PhotometricEnvironment {
        toward_sun: [0.0, 1.0, 0.0],
        sun_lux: 0.0,
        sun_colour: [1.0; 3],
        ambient_lux: [1200.0, 600.0, 300.0],
        background_radiance: [0.0; 3],
    };
    let mut scene = SyntheticScene::grid(1).expect("one cube");
    scene.view = support::aimed([0.0, 100.0, -3.0], [0.0, 0.0, 1.0]);
    scene.instances[0].current[3] = [0.0, 100.0, 0.0, 1.0];
    scene.instances[0].previous = scene.instances[0].current;
    scene.materials[0].base_rgba = [0.8, 0.7, 0.6, 1.0];
    let sun = Vec3::new(0.4, 0.8, -0.3).normalize().to_array();
    for (name, lighting, reference) in [
        (
            "diagnostic",
            FrameLighting::Diagnostic(SyntheticLight::default()),
            include_bytes!("data/light-diagnostic.rgba"),
        ),
        (
            "photometric",
            FrameLighting::Photometric(ambient_only),
            include_bytes!("data/light-photometric.rgba"),
        ),
        (
            "atmosphere",
            FrameLighting::Atmosphere(
                AtmosphereFrame::new(sun, 100_000.0, [1.0; 3], 0.0, 1).expect("air"),
            ),
            include_bytes!("data/light-atmosphere.rgba"),
        ),
    ] {
        let mut host = pollster::block_on(support::host_with_extent(
            &capabilities,
            &queue,
            scene.clone(),
            lighting,
            [32, 32],
        ));
        let pixels = host.capture_pixels().expect("scene pixels");
        if name == "photometric" {
            let center = (16 * 32 + 16) * 4;
            assert!(pixels[center..center + 3].iter().any(|&value| value != 0));
            assert_eq!(&pixels[..3], &[0; 3]);
        }
        assert_eq!(pixels.len(), reference.len(), "{name} image size");
        let difference = pixels
            .iter()
            .zip(reference)
            .position(|(actual, expected)| actual != expected);
        assert!(
            difference.is_none(),
            "{name} image differs first at byte {difference:?}"
        );
    }
}
