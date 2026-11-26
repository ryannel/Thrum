//! Haze scales only the solar part of the lent aerial decode.
mod support;
mod world_light_fixture;

use std::time::Duration;
use thrum_render::{
    atmosphere::AtmosphereFrame,
    present::SurfaceProfile,
    runtime::SurfaceOutput,
    synthetic::FrameLighting,
};
use world_light_fixture::{BINDINGS, prepare, prepare_host, probe};

fn clear(_: u32, _: u32, _: u32, _: u32) -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}
fn overlay(_: u32, _: u32, _: u32, _: u32) -> [f32; 4] {
    [0.0, 0.0, 0.0, 0.375]
}

async fn clear_exposure_follows_falling_sun(gpu: &thrum_test_support::Opened) {
    let frame = |solar_irradiance| {
        AtmosphereFrame::new([0.0, 1.0, 0.0], solar_irradiance, [1.0; 3], 40.0, 1)
            .expect("clear day")
    };
    let extent = [8, 8];
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let surface = Some(SurfaceProfile { format, extent });
    let output = support::drawable(&gpu.device, format, extent);
    let (base, _) = prepare_host(gpu, frame(4_000.0), None, surface).await;
    let (identity, _) = prepare_host(gpu, frame(4_000.0), Some(clear), surface).await;
    let mut hosts = [base, identity];
    for host in &mut hosts {
        let source = host.source_mut();
        source.view = source
            .view
            .with_incident_exposure(0.0, -12.0, 18.0)
            .expect("incident meter");
        host.enable_submitted_capture(1).expect("open capture");
    }
    let mut previous_ev = None;
    for (step, irradiance) in [(0, 4_000.0), (1, 2_000.0), (2, 1_000.0)] {
        let [base, identity] = hosts.each_mut().map(|host| {
            host.continuous_source_mut().lighting = FrameLighting::Atmosphere(frame(irradiance));
            assert!(
                host.drive(
                    Duration::from_millis(step * 16),
                    Some(&SurfaceOutput {
                        view: &output,
                        extent
                    }),
                )
                .expect("submit frame")
                .submitted
            );
            thrum_test_support::wait::until("clear-sky submission", Duration::from_secs(5), || {
                gpu.device.poll(wgpu::PollType::Poll).expect("poll Metal");
                host.try_submitted_image().expect("read submitted pixels")
            })
        });
        let base_exposure = base.exposure_gpu_words.expect("ordinary exposure");
        let overlay_exposure = identity.exposure_gpu_words.expect("overlay exposure");
        assert_eq!(base_exposure, overlay_exposure, "step {step}");
        assert_eq!(base.rgba, identity.rgba, "step {step}");
        let ev = f32::from_bits(base_exposure[2]);
        if let Some(previous_ev) = previous_ev {
            assert!(ev < previous_ev - 0.5, "fixture must change exposure");
        }
        previous_ev = Some(ev);
    }
}

#[test]
fn metal_clear_is_identity_and_clouds_scale_only_solar_scatter() {
    let Some(gpu) = thrum_test_support::open(
        "haze scale",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    assert_eq!(gpu.gpu.adapter.get_info().backend, wgpu::Backend::Metal);
    let query = format!(
        "{BINDINGS}
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>) {{
    let input = queries[id.x];
    let point = atmosphere.camera_altitude.xyz + vec3(0.0,0.0,input.x);
    let a = environment_aerial(point,vec2(0.5));
    let raw = aerial_sample(input.x,vec2(0.5));
    let start = 2u*id.x;
    output[start] = vec4(a.transmission,environment_light.sky_diffuse[1].w);
    output[start+1u] = vec4(a.scatter,raw.night);
}}"
    );
    let inputs = [
        [100.0, 0.0, 0.0, 0.0],
        [1000.0, 0.0, 0.0, 0.0],
        [25_000.0, 0.0, 0.0, 0.0],
    ];
    pollster::block_on(async {
        clear_exposure_follows_falling_sun(&gpu).await;
        for night in [false, true] {
            let frame = AtmosphereFrame::new([0.0, -0.2, 0.979_795_9], 70_000.0, [1.0; 3], 40.0, 1)
                .expect("twilight");
            let frame = if night { frame.with_nightglow() } else { frame };
            let (base, baseline_pixels) = prepare(&gpu, frame, None).await;
            let (identity, identity_pixels) = prepare(&gpu, frame, Some(clear)).await;
            assert_eq!(
                baseline_pixels, identity_pixels,
                "transparent sky with identity haze"
            );
            let (haze, _) = prepare(&gpu, frame, Some(overlay)).await;
            let sample = |env| probe(&gpu.device, &gpu.queue, env, &inputs, &query, 2);
            let clear = sample(&base);
            let one = sample(&identity);
            let hazy = sample(&haze);
            for ((expected, identity), adjusted) in clear
                .as_chunks::<2>()
                .0
                .iter()
                .zip(one.as_chunks::<2>().0)
                .zip(hazy.as_chunks::<2>().0)
            {
                assert_eq!(expected[0][..3], adjusted[0][..3], "transmittance");
                assert_eq!(identity[0][3].to_bits(), 1.0_f32.to_bits());
                assert_eq!(expected[1], identity[1], "identity aerial decode");
                assert_eq!(
                    expected[1][3].to_bits(),
                    adjusted[1][3].to_bits(),
                    "night term"
                );
                let scale = adjusted[0][3];
                assert!((scale - 0.375).abs() < 0.001);
                for c in 0..3 {
                    let night_radiance = expected[1][3];
                    let reference =
                        (expected[1][c] - night_radiance).mul_add(scale, night_radiance);
                    assert!((reference - adjusted[1][c]).abs() <= reference.abs() * 2e-6 + 1e-8);
                }
            }
            println!(
                "Metal: night={night} haze scale {} in three finite aerial samples",
                hazy[0][3]
            );
        }
        let volume = AtmosphereFrame::new([0.0, 1.0, 0.0], 70_000.0, [1.0; 3], 40.0, 1)
            .expect("noon")
            .with_view_volume();
        let (unscaled, _) = prepare(&gpu, volume, None).await;
        let (registered, _) = prepare(&gpu, volume, Some(overlay)).await;
        assert_eq!(
            probe(&gpu.device, &gpu.queue, &unscaled, &inputs, &query, 2),
            probe(&gpu.device, &gpu.queue, &registered, &inputs, &query, 2),
            "view-volume scenes keep their own solar in-scatter"
        );
    });
}
