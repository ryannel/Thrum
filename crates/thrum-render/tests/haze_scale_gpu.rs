//! Haze scales only the solar part of the lent aerial decode.
mod support;
mod world_light_fixture;

use thrum_render::atmosphere::AtmosphereFrame;
use world_light_fixture::{BINDINGS, prepare, probe};

fn clear(_: u32, _: u32, _: u32, _: u32) -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}
fn overlay(_: u32, _: u32, _: u32, _: u32) -> [f32; 4] {
    [0.0, 0.0, 0.0, 0.375]
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
