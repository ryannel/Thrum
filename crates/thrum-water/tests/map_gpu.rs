//! Camera-free Metal proof of the map sampled by the surface shader.
mod probe_support;
use probe_support as support;

#[test]
fn continuous_map_matches_the_cpu_at_scattered_points_and_shores() {
    let Some(test_gpu) = thrum_test_support::open(
        "continuous_map_matches_the_cpu_at_scattered_points_and_shores",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let map = support::fixture();
    let mut state = 0x1234_5678u32;
    let mut uniform = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 8) as f32 / 16_777_216.0
    };
    let mut points: Vec<_> = (0..4096)
        .map(|_| [uniform() * 82.0 - 3.0, uniform() * 73.0 - 3.0])
        .collect();
    // Independently bracket the changes in still-water owner along transects,
    // then probe either side of every crossing at sub-cell spacing.
    let owner = |p: [f32; 2]| {
        map.sample(p.map(f64::from))
            .filter(|a| a.depth > 0.0)
            .map(|a| a.body)
    };
    let mut crossings = [0; 4];
    for j in 0_i16..134 {
        let z = f32::from(j) * 0.5 + 0.173;
        let mut previous = [-1.0, z];
        for i in 0_i16..155 {
            let point = [f32::from(i) * 0.5 + 0.037, z];
            if owner(previous) != owner(point) {
                let [mut lo, mut hi] = [previous[0], point[0]];
                let before = owner(previous);
                for id in [before, owner(point)].into_iter().flatten() {
                    crossings[id] += 1;
                }
                for _ in 0..16 {
                    let mid = lo.midpoint(hi);
                    if owner([mid, z]) == before {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                for delta in [-0.09, -0.01, -0.001, 0.001, 0.01, 0.09] {
                    points.push([lo.midpoint(hi) + delta, z]);
                }
            }
            previous = point;
        }
    }
    assert!(crossings.iter().all(|&n| n >= 10), "{crossings:?}");
    let input: Vec<_> = points
        .iter()
        .map(|&[x, z]| [x, z, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        .collect();
    let mut seen = [0; 5];
    for (p, [answer, detail, _]) in points
        .into_iter()
        .zip(support::probe(&test_gpu, &map, &input))
    {
        let world = p.map(f64::from);
        let expected = map.sample(world);
        let body = expected.map_or(-1.0, |a| a.body as f32);
        assert_eq!(answer[0], body, "ownership at {p:?}");
        let bed = map.bed().height(world);
        seen[expected.map_or(4, |a| a.body)] += 1;
        assert_eq!(detail[2] != 0.0, bed.is_some());
        if let Some(bed) = bed {
            assert!((f64::from(answer[2]) - bed).abs() < support::EPS_METRES);
        }
        if let Some(expected) = expected {
            assert!((f64::from(answer[1]) - expected.row.level).abs() < support::EPS_METRES);
            if bed.is_some() {
                assert!((f64::from(answer[3]) - expected.depth).abs() < support::EPS_METRES);
            } else {
                assert!(answer[3] > 1e29);
            }
            for (actual, current) in detail.into_iter().zip(expected.row.current) {
                assert!(
                    (f64::from(actual) - current).abs() < support::EPS_CURRENT,
                    "{p:?}"
                );
            }
        }
    }
    assert!(seen.iter().all(|&n| n > 100), "{seen:?}");
}
