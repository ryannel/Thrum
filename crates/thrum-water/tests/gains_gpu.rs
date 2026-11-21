//! Fetch and depth gains as read from the map by the shared surface path.
use crate::probe_support;
use probe_support as support;
use thrum_sea_model::{Forcing, Spectrum, Swell};

#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the same significant heights are sent as f32 GPU inputs"
)]
fn gale_gains_preserve_the_exposed_sea_and_limit_each_layer_into_shallows() {
    let Some(test_gpu) = thrum_test_support::open(
        "gale_gains_preserve_the_exposed_sea_and_limit_each_layer_into_shallows",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let forcing = Forcing {
        wind_speed: 18.0,
        wind_direction: 0.0,
        fetch: 100_000.0,
        swells: vec![Swell {
            significant_height: 2.1,
            wavelength: 85.0,
            direction: 0.0,
        }],
    };
    let spectrum = Spectrum::build(&forcing).expect("gale");
    let [a, b, c] = spectrum
        .cascades
        .each_ref()
        .map(thrum_sea_model::Cascade::variance);
    let variances = [spectrum.variance() - a - b - c, a, b, c];
    let heights = variances.map(|v| 4.0 * v.sqrt());
    let mut map = support::fixture();
    map.set_forcing(&forcing).expect("map gains");
    let points: Vec<_> = [[-30.0, 12.0], [27.1, 20.4], [56.0, 30.0]]
        .into_iter()
        .chain((0_i16..100).map(|i| [f32::from(i) * 0.1, 12.0]))
        .filter(|p| map.sample(p.map(f64::from)).is_some())
        .collect();
    let inputs = [0.0, 1.0]
        .into_iter()
        .flat_map(|mutation| {
            points.iter().map(move |&[x, z]| {
                [
                    x,
                    z,
                    mutation,
                    0.0,
                    heights[0] as f32,
                    heights[1] as f32,
                    heights[2] as f32,
                    heights[3] as f32,
                ]
            })
        })
        .collect::<Vec<_>>();
    let probe = support::probe(&test_gpu, &map, &inputs);
    let mut failures_with_sea_gains = 0;
    for (i, point) in points.into_iter().enumerate() {
        let row = map.sample(point.map(f64::from)).expect("water or margin");
        let expected = thrum_water_model::limit(row.row, row.depth, variances, map.profile());
        for (b, hs) in heights.into_iter().enumerate() {
            let target = expected.gains[b] * hs;
            let actual = f64::from(probe[i][2][b]);
            if i == 0 {
                assert!((actual - hs).abs() < support::EPS_METRES);
            }
            assert!(
                (actual - target).abs() < support::EPS_METRES,
                "{point:?} band {b}: {actual} vs {target}"
            );
            let mutated = f64::from(probe[probe.len() / 2 + i][2][b]);
            failures_with_sea_gains += usize::from((mutated - target).abs() > 0.01);
        }
    }
    assert!(
        failures_with_sea_gains >= 4,
        "open-sea gains mutation must fail"
    );
}
