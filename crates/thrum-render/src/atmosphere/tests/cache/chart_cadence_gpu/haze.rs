use super::*;
use crate::atmosphere::{SunOpticalDepth, products::tests::observer};

fn cloud_field(depth: f32) -> SunOpticalDepth {
    SunOpticalDepth::new(
        [1, 1],
        &[depth],
        100.0,
        [-2000.0, -2000.0, 2000.0, 2000.0],
        0.02,
        1,
    )
    .unwrap()
}

#[test]
fn current_cloud_sun_factor_scales_aerial_between_diffuse_writes() {
    let Some(crate::test_device::Opened {
        gpu, device, queue, ..
    }) = crate::test_device::open(
        "Metal chart cadence haze",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction::default(),
    )
    else {
        return;
    };
    crate::timestamp_permit::hold();
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let air = sun(5.0, 90_000.0, [1.0; 3]).with_nightglow();
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.sky_overlay = true;
        scene.sun_shadows = false;
        scene.sun_optical_depth = Some(cloud_field(0.0));
        let mut spread = Probe::prepare(&caps, &queue, air, scene.clone()).await;
        let mut control = Probe::prepare(&caps, &queue, air, scene).await;
        for probe in [&spread, &control] {
            probe.overlay([0x4900, 0x4900, 0x4900, 0x3800]);
        }
        spread.draw(air);
        control.draw(air);
        let observe =
            |probe: &Probe<'_>| probe.prepared.environment_products_witness(&queue).unwrap();
        let spread_products = observe(&spread);
        let control_products = observe(&control);
        let probe = observer::Probe::new(
            &device,
            &spread_products,
            "let a = aerial_sample(750.0, q.xy);",
            [
                "vec4(aerial_radiance(a.scatter.rgb), a.night)",
                "vec4(a.transmission, 0.0)",
            ],
        );
        let sample = |products: &crate::atmosphere::EnvironmentProducts| {
            probe.sample(&device, &queue, products, &[[0.5, 0.5, 0.0, 0.0]], 0)[0]
        };

        spread.cadence(4);
        let stages = spread.draw(air);
        assert!(chart_names(&stages).any(|name| name == AtmosphereStage::Diffuse.name()));
        let before = sample(&spread_products);
        spread.scene.sun_optical_depth = Some(cloud_field(2.0));
        control.scene.sun_optical_depth = Some(cloud_field(2.0));
        let stages = spread.draw(air);
        assert!(!chart_names(&stages).any(|name| name == AtmosphereStage::Diffuse.name()));
        control.draw(air);
        let after = sample(&spread_products);
        let expected = sample(&control_products);
        assert_eq!(after[1].map(f32::to_bits), expected[1].map(f32::to_bits));
        assert_eq!(after[0][3].to_bits(), expected[0][3].to_bits());
        assert_eq!(after[1].map(f32::to_bits), before[1].map(f32::to_bits));
        assert_eq!(after[0][3].to_bits(), before[0][3].to_bits());
        for band in 0..3 {
            assert!(
                (after[0][band] - expected[0][band]).abs()
                    <= 0.001 * expected[0][band].abs().max(1.0)
            );
            assert!(after[0][band] < before[0][band], "{before:?} -> {after:?}");
        }
    });
}
