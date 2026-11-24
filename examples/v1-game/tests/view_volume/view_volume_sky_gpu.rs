//! The view volume’s terminal-pair composition on Metal.
#[expect(
    dead_code,
    reason = "sky composition reads the image and terminal pair"
)]
mod fixture;

use fixture::{SCATTER_GAIN, STORAGE_EXPOSURE, SUN_LUX};

fn compare(actual: f32, expected: f32) -> bool {
    // Two roundings of the scene's fp16 output plus the stored correction.
    (actual - expected).abs() <= 0.0015 * expected.abs() + 0.000_02
}

#[test]
fn metal_sky_applies_the_sampled_terminal_pair_exactly_once() {
    let Some(gpu) = fixture::Gpu::open(7, "view volume terminal pair") else {
        return;
    };
    let clear = fixture::scene(false, false, false);
    let (mut legacy, mut legacy_target) = gpu.prepare(&clear, fixture::light(false, 1));
    let legacy = fixture::render(
        &gpu,
        &mut legacy,
        &mut legacy_target,
        &clear,
        fixture::light(false, 1),
    );
    let l = legacy.center();
    for (caster, cloud) in [(false, false), (true, false), (false, true)] {
        let scene = fixture::scene(caster, cloud, false);
        let light = fixture::light(true, 1);
        let (mut prepared, mut target) = gpu.prepare(&scene, light);
        let actual = fixture::render(&gpu, &mut prepared, &mut target, &scene, light);
        let pair = actual.terminal();
        assert_eq!(pair[3].to_bits(), 1.0_f32.to_bits());
        let sky = actual.center();
        if !caster && !cloud {
            assert_eq!(
                pair[..3]
                    .iter()
                    .map(|lane| *lane == 0.0)
                    .collect::<Vec<_>>(),
                [true; 3]
            );
            assert_eq!(sky.map(f32::to_bits), l.map(f32::to_bits));
            continue;
        }
        let delta =
            std::array::from_fn::<_, 3, _>(|i| pair[i] * SUN_LUX * STORAGE_EXPOSURE / SCATTER_GAIN);
        assert!(
            delta.iter().all(|value| *value < 0.0),
            "caster={caster} cloud={cloud}: {delta:?}"
        );
        for i in 0..3 {
            let expected = l[i] + delta[i];
            assert!(
                compare(sky[i], expected),
                "caster={caster} cloud={cloud} channel={i}: {} vs {expected}",
                sky[i]
            );
        }
        let misses =
            |scale: f32| (0..3).any(|i| !compare(l[i] + scale * delta[i], l[i] + delta[i]));
        assert!(misses(0.0), "clear sky falsifier");
        assert!(misses(2.0), "two applications falsifier");
    }
}
