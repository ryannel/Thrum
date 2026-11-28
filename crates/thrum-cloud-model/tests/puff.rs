//! Behaviour of the opt-in puff form through the game's public query.
use crate::support::{close, parameters};
use thrum_cloud_model::{Column, Density, Field, Parameters, Profile, PuffProfile, State, Type};

const BASE: f64 = 700.0;
const PITCH: f64 = 1300.0;
const CORE: [f64; 2] = [1_070.286_263_176_24, 1_098.399_745_742_790_4];
const OVERHANG: [f64; 2] = [980.0, 640.0];
const GRAZE: [f64; 2] = [898.272_380_324, 694.157_133_144];
const MIXED: [f64; 2] = [960.0, 640.0];
const CONVERGENCE: f64 = 0.8;

fn model(deck: f64, varying_type: bool) -> Field {
    model_with_fade(deck, varying_type, 100.0)
}

fn model_with_fade(deck: f64, varying_type: bool, fade: f64) -> Field {
    let heaped = Type {
        base: BASE,
        depth: [260.0, 950.0],
        liquid_water: 0.0003,
        droplet_radius: 9e-6,
        deck,
        profile: [0.2, 0.3],
    };
    Field::new_puff(
        7,
        PuffProfile {
            weather: Profile {
                pitch: PITCH,
                period: 1600.0,
                turret_pitch: 340.0,
                turret_least: 0.18,
                turret_fade: 0.12,
                detail_mean: 0.5,
                erosion: 0.3,
            },
            fade,
            erosion: 0.85,
        },
        vec![
            Type {
                base: BASE + if varying_type { 400.0 } else { 0.0 },
                deck: deck + if varying_type { 0.85 } else { 0.0 },
                ..heaped
            },
            heaped,
        ],
    )
    .expect("puff profile")
}

fn at(time: f64, weather: Parameters) -> State {
    let mut state = State::new(weather, 10.0, 0).expect("valid weather");
    if time > 0.0 {
        state.step(1, time).expect("commit");
    }
    state
}

fn mature(field: &Field, weather: Parameters) -> State {
    let cell = field.cell(0, 0, 0);
    at(cell.birth + 0.5 * cell.life, weather)
}

fn point(air: [f64; 2], y: f64) -> [f64; 3] {
    [air[0], y, air[1]]
}

fn density(field: &Field, state: &State, air: [f64; 2], y: f64, fidelity: Density) -> f64 {
    field.density(point(air, y), state, fidelity, |_| None, 1.0)
}

#[test]
fn puff_clear_and_cloud_share_the_condensation_level() {
    let field = model(0.0, true);
    let clear = mature(&field, parameters(0.0, 0.75));
    let cloudy = mature(&field, parameters(1.0, 0.75));
    let clear = field.query(point(CORE, 3300.0), &clear);
    let cloudy = field.query(point(CORE, 0.0), &cloudy);
    assert_eq!(clear.coverage, 0.0);
    assert_eq!(clear.optical_depth, 0.0);
    assert_eq!(clear.age, 0.0);
    assert_eq!(clear.top, clear.base);
    assert_eq!(clear.base, 800.0);
    assert_eq!(cloudy.base, clear.base);
    assert!(cloudy.top > cloudy.base && cloudy.coverage > 0.0);
}

#[test]
fn puff_travel_is_bit_exact_for_column_and_density() {
    let field = model(0.0, false);
    let weather = parameters(1.0, 1.0);
    let time = mature(&field, weather).time();
    for wind in [[0.25, 0.5], [-180_000.0, 360_000.0], [12.5, -16.0]] {
        let moved = at(time, Parameters { wind, ..weather });
        let held = at(time, weather);
        let [x, z] = moved.travel();
        let world = [CORE[0] + x, BASE + 200.0, CORE[1] + z];
        let air = [world[0] - x, world[1], world[2] - z];
        let a = field.query(world, &moved);
        let b = field.query(air, &held);
        assert!(a.coverage > 0.0);
        assert_eq!(values(a).map(f64::to_bits), values(b).map(f64::to_bits));
        assert_eq!(a.age.to_bits(), b.age.to_bits());
        assert_eq!(a.base.to_bits(), b.base.to_bits());
        assert_eq!(a.kind.to_bits(), b.kind.to_bits());
        assert_eq!(a.profile.map(f64::to_bits), b.profile.map(f64::to_bits));
        let sample = |point, state| field.density(point, state, Density::Full, |_| None, 1.0);
        assert_eq!(
            sample(world, &moved).to_bits(),
            sample(air, &held).to_bits()
        );
    }
}

fn values(column: Column) -> [f64; 4] {
    [
        column.optical_depth,
        column.coverage,
        column.top,
        column.top - column.base,
    ]
}

fn largest<const N: usize>(samples: impl Iterator<Item = [f64; N]>) -> [f64; N] {
    let mut maximum = [0.0_f64; N];
    let mut previous: Option<[f64; N]> = None;
    for next in samples {
        if let Some(previous) = previous {
            for i in 0..N {
                maximum[i] = maximum[i].max((next[i] - previous[i]).abs());
            }
        }
        previous = Some(next);
    }
    maximum
}

fn converges(coarse: f64, fine: f64) -> bool {
    coarse > 0.0 && fine < CONVERGENCE * coarse
}

#[expect(
    clippy::print_stdout,
    reason = "records convergence measurements in the target proof"
)]
fn check_halvings<const N: usize>(series: [[f64; N]; 3]) {
    println!("largest changes at each spacing: {series:?}");
    for pair in series.windows(2) {
        for j in 0..N {
            assert!(converges(pair[0][j], pair[1][j]), "{pair:?} component {j}");
        }
    }
}

#[test]
fn puff_temporal_continuity_through_a_day_and_weather_ramps() {
    let field = model(0.0, true);
    for (ramp_coverage, ramp_kind) in [(false, false), (true, false), (false, true)] {
        let points = [CORE, [CORE[0] + 32.0, CORE[1] + 24.0]];
        // The sample grids are nested. Query each committed state once per
        // point and use the corresponding subsequence for each spacing.
        let day: Vec<_> = (0..=86_400 / 8)
            .map(|i| {
                let t = f64::from(i * 8);
                let coverage = if ramp_coverage {
                    0.42 + 0.32 * (t / 4200.0).sin()
                } else if ramp_kind {
                    0.45
                } else {
                    0.83
                };
                let kind = if ramp_kind {
                    0.52 + 0.48 * (t / 11_000.0).sin()
                } else {
                    1.0
                };
                let state = at(t, parameters(coverage, kind));
                points.map(|air| field.query(point(air, 0.0), &state))
            })
            .collect();
        for (point_index, _) in points.into_iter().enumerate() {
            let series = [32, 16, 8].map(|step| {
                let mut births = 0;
                let mut deaths = 0;
                let mut previous = false;
                let samples = day.iter().step_by(step / 8).enumerate().map(|(i, cols)| {
                    let column = cols[point_index];
                    let cloudy = column.coverage > 0.0;
                    births += usize::from(i > 0 && !previous && cloudy);
                    deaths += usize::from(previous && !cloudy);
                    previous = cloudy;
                    values(column)
                });
                let maximum = largest(samples);
                assert!(births > 0 && deaths > 0, "{births} births, {deaths} deaths");
                println!("step {step}s: {births} births, {deaths} deaths");
                maximum
            });
            check_halvings(series);
        }
    }
}

#[test]
fn puff_temporal_continuity_rejects_a_cell_switched_on_between_samples() {
    let field = model(0.0, false);
    let cell = field.cell(0, 0, 0);
    let weather = parameters(1.0, 1.0);
    let on = values(field.query(point(CORE, 0.0), &mature(&field, weather)));
    let off = values(field.query(point(CORE, 0.0), &at(0.0, parameters(0.0, 1.0))));
    let switch = 0.137_123;
    let measure = |step: f64, switched: bool| {
        largest((0..=300).map(|i| {
            let phase = switch + (f64::from(i) - 150.5) * step;
            let t = cell.birth + phase * cell.life;
            if switched {
                if phase < switch { off } else { on }
            } else {
                values(field.query(point(CORE, 0.0), &at(t, weather)))
            }
        }))
    };
    let steps = [0.0008, 0.0004, 0.0002];
    check_halvings(steps.map(|step| measure(step, false)));
    let jumped = steps.map(|step| measure(step, true));
    println!("switched cell changes: {jumped:?}");
    for pair in jumped.windows(2) {
        for (&coarse, &fine) in pair[0].iter().zip(&pair[1]) {
            assert!(coarse > 0.0);
            assert!(!converges(coarse, fine));
        }
    }
}

#[test]
fn puff_spatial_continuity_through_overlapping_cells() {
    let field = model(0.0, false);
    let state = mature(&field, parameters(1.0, 1.0));
    let sample = |x| field.query([x, 0.0, 825.0], &state);
    let mut overlapping_winners = false;
    let mut previous = sample(1200.0);
    for i in 1..=600 {
        let next = sample(1200.0 + f64::from(i) * 0.5);
        // A change of dominant cell while coverage stays substantial.
        overlapping_winners |= next.age.to_bits() != previous.age.to_bits()
            && next.coverage.min(previous.coverage) > 0.1
            && (next.coverage - previous.coverage).abs() < 0.005;
        previous = next;
    }
    assert!(
        overlapping_winners,
        "line must cross the two cells' shared area"
    );
    let series = [4, 2, 1].map(|step| {
        largest((0..=320 / step).map(|i| {
            let c = sample(1200.0 + f64::from(i * step));
            [c.optical_depth, c.coverage]
        }))
    });
    check_halvings(series);
}

#[test]
fn puff_column_bound_covers_core_overhang_and_substep_graze() {
    let field = model(0.0, false);
    let state = mature(&field, parameters(1.0, 1.0));
    for (name, air) in [("core", CORE), ("overhang", OVERHANG), ("graze", GRAZE)] {
        let column = field.query(point(air, 0.0), &state);
        assert!(column.coverage > 0.0);
        let mut first = f64::INFINITY;
        let mut last = 0.0_f64;
        let mut sums = [0.0; 3];
        let mut y = BASE + 0.05;
        while y < column.top + 0.1 {
            for (i, fidelity) in [Density::Uneroded, Density::Coarse, Density::Full]
                .into_iter()
                .enumerate()
            {
                let sigma = density(&field, &state, air, y, fidelity);
                if i == 0 && sigma > 0.0 {
                    first = first.min(y);
                    last = last.max(y);
                }
                sums[i] += 0.1 * sigma;
            }
            y += 0.1;
        }
        let bound = column.optical_depth;
        println!("{name} {first:.2}–{last:.2} m: bound {bound}, sums {sums:?}");
        assert!(sums[0] > 0.0 && sums[0] <= bound && bound <= 1.06 * sums[0]);
        assert!(sums[1] <= bound && sums[2] <= bound);
        if name == "overhang" {
            assert!(first > BASE + 50.0);
        }
        if name == "graze" {
            assert!((10.0..=50.0).contains(&(last - first)));
            // This witness crosses one sphere at its rim. Check the public
            // density against the sphere across the chord, independently
            // of both the column bound and the numerical sum.
            let centre = 868.653_113_128;
            let radius: f64 = 108.842_567_896;
            let half: f64 = 17.0;
            for offset in [-17.1_f64, -13.0, -8.0, 0.0, 8.0, 13.0, 17.1] {
                let distance = (radius.powi(2) - half.powi(2) + offset.powi(2)).sqrt();
                let expected = 0.05 * ((radius - distance) / 100.0).max(0.0).powf(0.6);
                close(
                    density(&field, &state, air, centre + offset, Density::Uneroded),
                    expected,
                    1e-10,
                );
            }
            let mut y = BASE;
            let mut undersampled = 0.0;
            while y <= column.top + 50.0 {
                undersampled += 50.0 * density(&field, &state, air, y, Density::Uneroded);
                y += 50.0;
            }
            println!("graze at 50 m spacing: {undersampled}");
            assert_eq!(undersampled, 0.0);
            assert!(undersampled < sums[0]);
        }
    }
}

#[test]
fn puff_cloudy_share_never_falls_with_coverage() {
    let field = model(0.0, false);
    let weather = parameters(1.0, 1.0);
    let time = mature(&field, weather).time();
    let mut previous = 0;
    for i in 0..=20 {
        let state = at(time, parameters(f64::from(i) / 20.0, 1.0));
        let mut cloudy = 0;
        for x in -10..10 {
            for z in -10..10 {
                let air = [(f64::from(x) + 0.48) * PITCH, (f64::from(z) + 0.41) * PITCH];
                cloudy += usize::from(field.query(point(air, 0.0), &state).coverage > 0.0);
            }
        }
        assert!(
            cloudy >= previous,
            "coverage index {i}: {cloudy} < {previous}"
        );
        previous = cloudy;
    }
    assert!(
        previous > 20,
        "the square must contain a real spread of cells"
    );
}

#[test]
fn puff_overhang_retains_air_between_cloudy_heights_and_above_deck() {
    let field = model(0.0, false);
    let full = parameters(1.0, 1.0);
    let state = mature(&field, full);
    let at_overhang = |h| density(&field, &state, OVERHANG, BASE + h, Density::Uneroded);
    assert!(at_overhang(169.0) > 0.0);
    assert_eq!(at_overhang(212.0), 0.0);
    assert!(at_overhang(280.0) > 0.0);

    let profile = |field, state, y| density(field, state, MIXED, y, Density::Uneroded);
    let mixed = model(0.16, false);
    let shallow = mixed.procedural_column(MIXED, state.time(), full, 0.0);
    let deck_y = shallow.base.midpoint(shallow.top);
    assert!(shallow.coverage > 0.0 && deck_y < shallow.top);
    close(
        profile(&mixed, &state, deck_y),
        shallow.coverage * (0.05),
        1e-13,
    );
    let gap_y = BASE + 60.0;
    assert!(gap_y > shallow.top);
    assert_eq!(profile(&mixed, &state, gap_y), 0.0);
    let puff_y = BASE + 185.0;
    let without_deck = at(state.time(), parameters(0.45, 1.0));
    assert_eq!(
        mixed
            .procedural_column(MIXED, state.time(), without_deck.parameters(), 0.0)
            .coverage,
        0.0
    );
    let puff_only = profile(&mixed, &without_deck, puff_y);
    assert!(puff_only > 0.0);
    close(profile(&mixed, &state, puff_y), puff_only, 1e-13);

    let column = mixed.query(point(MIXED, 0.0), &state);
    let extent = column.top - column.base;
    let integral: f64 = (0..7000)
        .map(|i| {
            let step = extent / 7000.0;
            profile(&mixed, &state, BASE + (f64::from(i) + 0.5) * step) * step
        })
        .sum();
    assert!(integral > shallow.optical_depth);
    assert!(column.optical_depth >= integral && column.optical_depth <= integral * 1.06);
}

// Sphere and noise witnesses from this seed's generated geometry, in metres
// relative to the condensation base. Keep the oracle independent of the
// model's depth, shape curve and column integration.
struct PuffWitness {
    centre: [f64; 3],
    radius: f64,
}

impl PuffWitness {
    fn depth(&self, point: [f64; 3]) -> f64 {
        self.radius - distance(self.centre, point)
    }

    fn profile(&self, point: [f64; 3]) -> f64 {
        (self.depth(point).min(point[1]) / 100.0).clamp(0.0, 1.0)
    }
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

fn density_at(field: &Field, state: &State, p: [f64; 3], fidelity: Density) -> f64 {
    density(field, state, [p[0], p[2]], BASE + p[1], fidelity)
}

#[test]
fn puff_depth_uses_the_deepest_sphere_at_a_join() {
    let field = model(0.0, false);
    let state = mature(&field, parameters(1.0, 1.0));
    let a = PuffWitness {
        centre: [1_305.780_303_115, 192.086_630_642, 1_007.314_871_604],
        radius: 126.899_081_723,
    };
    let b = PuffWitness {
        centre: [1_444.053_304_736, 103.328_054_092, 1_065.842_945_184],
        radius: 180.692_498_394,
    };
    let fraction = 0.35;
    let point = std::array::from_fn(|i| a.centre[i] + (b.centre[i] - a.centre[i]) * fraction);
    let depths = [a.depth(point), b.depth(point)];
    let deepest = depths[0].max(depths[1]);
    assert!(depths[0] > 0.0 && depths[1] > 0.0 && deepest < 100.0 && point[1] > 100.0);

    // This point is on the axis between the centres. The nearest exposed
    // point of the pair is on the seam circle. Other spheres in the cluster
    // can bury that seam, making this a lower bound on the exposed depth.
    let separation = distance(a.centre, b.centre);
    let seam = (a.radius.powi(2) - b.radius.powi(2) + separation.powi(2)) / (2.0 * separation);
    let seam_radius = (a.radius.powi(2) - seam.powi(2)).sqrt();
    let exposed = (seam - fraction * separation)
        .hypot(seam_radius)
        .min(point[1]);
    assert!(exposed - deepest >= 50.0);
    let strength = 0.996_254_780_415;
    let expected = 0.05 * (strength * deepest / 100.0).powf(0.6);
    let wrong = 0.05 * (strength * (exposed / 100.0).min(1.0)).powf(0.6);
    let got = density_at(&field, &state, point, Density::Uneroded);
    close(got, expected, 3e-9);
    assert!((got - wrong).abs() > 0.001);
}

fn threshold(noise: [f64; 2]) -> f64 {
    0.85 * (0.72 * (1.0 - noise[0]) + 0.28 * (1.0 - noise[1]))
}

fn eroded(profile: f64, t: f64) -> f64 {
    ((profile - t) / (1.0 - t)).clamp(0.0, 1.0).powf(0.6)
}

#[test]
fn puff_density_uses_one_weighted_heaped_profile_before_erosion() {
    let field = model(0.0, false);
    let state = mature(&field, parameters(1.0, 1.0));
    let partial = PuffWitness {
        centre: [538.857_692_256, 123.493_949_473, 2_735.584_550_570],
        radius: 104.074_587_482,
    };
    let part_strength = 0.812_368_346_605;
    let one = partial.centre;
    let raw = partial.profile(one);
    let near = PuffWitness {
        centre: [1_139.013_734_053, 291.136_078_469, 971.660_999_019],
        radius: 97.454_266_948,
    };
    let far = PuffWitness {
        centre: [1_214.182_383_506, 299.053_143_988, 985.767_203_891],
        radius: 59.566_265_987,
    };
    let overlap = [1_175.669_617_556, 292.481_712_684, 982.923_942_871];
    let weights = [0.996_254_780_415, 1.0_f64];
    let profiles = [near.profile(overlap), far.profile(overlap)];
    assert!(profiles[0] > 0.0 && profiles[1] > 0.0 && weights[0] < weights[1]);
    let heaped = (weights[0] * profiles[0]).max(weights[1] * profiles[1]);

    for (fidelity, part_fine, overlap_fine) in [
        (Density::Full, 0.446_779_528_165, 0.392_406_892_594),
        (Density::Coarse, 0.5, 0.5),
    ] {
        let t = threshold([0.806_329_817_028, part_fine]);
        let actual = density_at(&field, &state, one, fidelity);
        close(actual, 0.05 * eroded(part_strength * raw, t), 3e-9);
        let wrong = 0.05 * part_strength * eroded(raw, t);
        assert!((actual - wrong).abs() > 0.0001);

        let t = threshold([0.705_789_477_347, overlap_fine]);
        let actual = density_at(&field, &state, overlap, fidelity);
        close(actual, 0.05 * eroded(heaped, t), 3e-9);
    }
}

#[test]
fn puff_cluster_cache_distinguishes_fade_lengths() {
    let ordinary = model(0.0, false);
    let small = model_with_fade(0.0, false, 50.0);
    let weather = parameters(1.0, 1.0);
    let state = mature(&ordinary, weather);
    let sample = point(CORE, 0.0);

    // Visit the same cache slot at a different time between cold reads.
    ordinary.query(sample, &at(state.time() + 1.0, weather));
    let expected = small.query(sample, &state);
    ordinary.query(sample, &at(state.time() + 1.0, weather));
    let other = ordinary.query(sample, &state);
    assert!(expected.top < other.top);
    assert_eq!(small.query(sample, &state), expected);
}
