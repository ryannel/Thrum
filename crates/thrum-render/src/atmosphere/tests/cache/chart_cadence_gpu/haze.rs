use super::*;
use crate::atmosphere::{SunOpticalDepth, products::tests::observer};
use mixed_overlay::light_edit::OneOverlay;

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

fn classifications(probe: &Probe<'_>) -> [u32; 2] {
    let device = probe.caps.device();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let copy = Readback::buffer(
        device,
        &mut encoder,
        probe.prepared.overlay_classification_witness(),
        ..,
    );
    let bytes = copy.wait(device, probe.queue.submit([encoder.finish()]));
    [4, 12].map(|offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()))
}

#[test]
fn governed_haze_and_diffuse_share_one_classification_for_a_registered_write() {
    let Some(crate::test_device::Opened {
        gpu, device, queue, ..
    }) = crate::test_device::open(
        "Metal governed overlay classification",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction::default(),
    )
    else {
        return;
    };
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let air = sun(30.0, 90_000.0, [1.0; 3]);
        let dim = sun(30.0, 45_000.0, [1.0; 3]);
        for cloudy in [false, true] {
            let mut scene = SyntheticScene::grid(1).unwrap();
            scene.sky_overlay = true;
            scene.sun_shadows = false;
            scene.sun_optical_depth = Some(cloud_field(0.0));
            let mut spread = Probe::prepare(&caps, &queue, air, scene).await;
            let mut overlay = OneOverlay::register(&mut spread, cloudy);
            spread.cadence(8);
            overlay.submit_frame(&mut spread, air);
            assert_eq!(classifications(&spread), [u32::from(cloudy), 1]);
            let products = spread
                .prepared
                .environment_products_witness(&queue)
                .unwrap();
            let probe = observer::Probe::new(
                &device,
                &products,
                "let a = aerial_sample(750.0, q.xy);",
                ["vec4(aerial_radiance(a.scatter.rgb), a.night)"],
            );
            let read =
                || probe.sample(&device, &queue, &products, &[[0.5, 0.5, 0.0, 0.0]], 0)[0][0];
            let before = read();
            overlay.runs = false;
            for submission in 0..8 {
                if submission == 0 {
                    spread.scene.sun_optical_depth = Some(cloud_field(2.0));
                }
                // The first edit changes only the live haze. The light edit
                // on the last submission makes diffuse due at fixed overlay.
                overlay.submit_frame(&mut spread, if submission < 7 { air } else { dim });
                assert_eq!(classifications(&spread), [u32::from(cloudy), 1]);
                if submission == 0 && cloudy {
                    let after = read();
                    assert!(after[..3].iter().zip(before).all(|(&new, old)| new < old));
                    assert_eq!(after[3].to_bits(), before[3].to_bits());
                }
            }
            assert_eq!(overlay.executions, 1);
        }
    });
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
