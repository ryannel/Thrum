//! The view volume’s direct-sun shadow and forward decoder on Metal.
mod fixture;

use fixture::{SCATTER_GAIN, STORAGE_EXPOSURE, SUN_LUX, Witness};

fn sample(witness: &Witness, depth: f32) -> ([f32; 4], [f32; 4]) {
    let columns = (witness.grid[0] * witness.grid[1]) as usize * 8;
    let layers = witness.grid[2] - 2;
    let near = layers / 2;
    let coord = (depth / 500.0).sqrt() * f32::from(u16::try_from(near).expect("near layers"));
    let upper = u16::try_from(
        (0..near)
            .find(|layer| {
                coord <= f32::from(u16::try_from(*layer + 1).expect("validated fixture data"))
            })
            .expect("near depth")
            + 1,
    )
    .expect("validated fixture data");
    let layer_depth = |layer| {
        500.0
            * (f32::from(layer) / f32::from(u16::try_from(near).expect("validated fixture data")))
                .powi(2)
    };
    let start = layer_depth(upper - 1);
    let blend = (depth - start) / (layer_depth(upper) - start);
    let interpolate = |bytes: &[u8]| {
        let low = witness.volume_center(&bytes[columns * usize::from(upper - 1)..]);
        let high = witness.volume_center(&bytes[columns * usize::from(upper)..]);
        std::array::from_fn(|i| low[i] + (high[i] - low[i]) * blend)
    };
    (
        interpolate(&witness.scatter),
        interpolate(&witness.transmission),
    )
}

#[test]
fn metal_shadow_and_cloud_dim_only_the_solar_source_and_forward_reads_it_once() {
    let Some(gpu) = fixture::Gpu::open(7, "shadowed view volume") else {
        return;
    };
    let light = fixture::light(true, 1);
    let render = |caster, cloud| {
        let scene = fixture::scene(caster, cloud, true);
        let (mut prepared, mut target) = gpu.prepare(&scene, light);
        fixture::render(&gpu, &mut prepared, &mut target, &scene, light)
    };
    let clear = render(false, false);
    let terrain = render(true, false);
    let cloud = render(false, true);
    let opaque = {
        let mut scene = fixture::scene(false, true, true);
        scene.sun_optical_depth = Some(fixture::sun_path(64.0, -55.0));
        let (mut prepared, mut target) = gpu.prepare(&scene, light);
        fixture::render(&gpu, &mut prepared, &mut target, &scene, light)
    };
    assert!(
        clear.transmission == terrain.transmission && clear.transmission == cloud.transmission,
        "shadow cannot change transmission or absolute night in-scatter"
    );
    let (lit, _) = sample(&clear, 28.0);
    let (shadowed, transmission) = sample(&terrain, 28.0);
    let (clouded, _) = sample(&cloud, 28.0);
    let (blocked, _) = sample(&opaque, 28.0);
    let sky = terrain.center();
    for i in 0..3 {
        assert!(
            shadowed[i] < lit[i] && clouded[i] < lit[i],
            "both shadow producers darken air"
        );
        let expected_cloud = blocked[i] + (lit[i] - blocked[i]) * (-1.0_f32).exp();
        assert!(
            (clouded[i] - expected_cloud).abs() <= 0.003 * lit[i],
            "the sampled optical depth attenuates single scatter at the cell"
        );
        let expected =
            STORAGE_EXPOSURE * (SUN_LUX * shadowed[i] / SCATTER_GAIN + transmission[3] / 65_536.0);
        assert!(
            (sky[i] - expected).abs() <= expected.abs() * 0.006 + 0.000_02,
            "black receiver at 28 m must display the volume's own scatter {sky:?} expected {expected}"
        );
    }
    let from = 14.0;
    let to = 26.0;
    let terrain_a = sample(&terrain, from).0;
    let terrain_b = sample(&terrain, to).0;
    let opaque_a = sample(&opaque, from).0;
    let opaque_b = sample(&opaque, to).0;
    for i in 0..3 {
        let known_blocked = opaque_b[i] - opaque_a[i];
        let actual = terrain_b[i] - terrain_a[i];
        assert!(
            (actual - known_blocked).abs() <= 0.008 * lit[i],
            "between 14 and 26 m the cascade admits no single scatter: {actual} vs {known_blocked}"
        );
    }
    // 55 m is the field boundary. The shadow deficit plateaus beyond it,
    // while the unshadowed transport continues to grow with range.
    let (a, _) = sample(&cloud, 83.0);
    let (b, _) = sample(&cloud, 132.0);
    let (clear_a, _) = sample(&clear, 83.0);
    let (clear_b, _) = sample(&clear, 132.0);
    for i in 0..3 {
        let delta = clear_a[i] - a[i];
        assert!(delta > 0.0);
        assert!((delta - (clear_b[i] - b[i])).abs() <= delta * 0.12);
    }
}
