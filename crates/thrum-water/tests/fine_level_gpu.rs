//! Submitted Metal fine-level state, including the spectrum kept by each level.

mod interaction_fixture;

use interaction_fixture::{Fixture, terrain};
use std::f64::consts::TAU;
use thrum_sea_model::Sample;
use thrum_water::{
    interaction::{BodyPressure, Limits, Witness},
    query::{BodyQuery, QueryPoint},
};
use thrum_water_model::{
    Bed, Body, BodyKind, CAPILLARY_COEFFICIENT, Hollows, Impulse, Map, Stage, Surface,
};

const SIDE: usize = Witness::SIDE;
const CELL: f64 = 0.02;

fn follow(fixture: &Fixture, tick: u64, place: [f64; 2]) {
    fixture.mailbox().post_bodies(
        tick,
        Some(0),
        vec![BodyPressure {
            id: 0,
            place,
            heading: 0.0,
            footprint: [0.1; 2],
            force: 0.0,
        }],
    );
}

fn step(fixture: &mut Fixture, tick: u64) {
    follow(fixture, tick, [0.0, 0.0]);
    fixture.draw(tick);
}

fn impact(fixture: &Fixture, tick: u64, x: f64, vertical: f64) {
    fixture.mailbox().post_impulse(Impulse {
        id: tick,
        place: [x, 0.0],
        vertical,
        radius: 0.055,
        tick,
    });
}

fn mode(plane: &[[f32; 2]], bin: usize, channel: usize) -> f64 {
    let (sin, cos) = (0..SIDE).fold((0.0, 0.0), |(sin, cos), x| {
        let sum: f64 = plane
            .iter()
            .skip(x)
            .step_by(SIDE)
            .map(|v| f64::from(v[channel]))
            .sum();
        let angle = TAU * (x * bin) as f64 / SIDE as f64;
        (sin + sum * angle.sin(), cos + sum * angle.cos())
    });
    sin.hypot(cos)
}

#[test]
fn source_wavelength_has_one_level() {
    // 0.64 m fits exactly 100 cycles on the near grid and eight on the fine grid.
    // 2.56 m fits 25 cycles on the near grid and two on the fine grid.
    let amplitudes = |fine| {
        let map = terrain(|_| -4.0);
        let fixture = if fine {
            Fixture::new_fine(map)
        } else {
            Fixture::new(map, Limits::default())
        };
        let mut fixture = fixture?;
        step(&mut fixture, 0);
        impact(&fixture, 1, 0.0, 0.6);
        step(&mut fixture, 1);
        let field = fixture.read();
        let near = [mode(field.level(0), 100, 1), mode(field.level(0), 25, 1)];
        let fine = fine.then(|| [mode(field.fine(), 8, 1), mode(field.fine(), 2, 1)]);
        Some((near, fine))
    };
    let (Some((original, _)), Some((near, fine))) = (amplitudes(false), amplitudes(true)) else {
        return;
    };
    let fine = fine.expect("submitted fine plane");
    eprintln!(
        "0.64 m / 2.56 m potential amplitudes, original near {original:?}, split near {near:?}, fine {fine:?}"
    );
    assert!(original[0] > 0.001 && original[1] > 0.001);
    assert!(
        near[0] < original[0] * 0.02,
        "near state retained the 0.64 m wave"
    );
    assert!((near[1] / original[1] - 1.0).abs() < 0.02);
    // A raw DFT includes the number of samples per square metre. Use
    // area-weighted amplitudes to compare the same physical source.
    let area = (f64::from(Witness::FINE_CELL_METRES) / f64::from(Witness::CELL_METRES[0])).powi(2);
    assert!((area * fine[0] / original[0] - 1.0).abs() < 0.10);
    assert!(area * fine[1] < near[1] * 0.02);
}

#[test]
fn fine_catch_up_advances_two_separate_ticks() {
    let run = |hitch| {
        let mut fixture = Fixture::new_fine(terrain(|_| -0.03))?;
        step(&mut fixture, 0);
        if hitch {
            // A dropped source must not appear on the fine level.
            impact(&fixture, 3, -0.45, 1.0);
            impact(&fixture, 5, 0.0, 0.6);
            impact(&fixture, 6, 0.1, 0.2);
            step(&mut fixture, 6);
            let report = fixture
                .report
                .lock()
                .expect("report")
                .interaction
                .unwrap()
                .fine
                .unwrap();
            assert_eq!(report.steps, 2);
            assert_eq!(report.dropped_ticks, 4);
        } else {
            impact(&fixture, 1, 0.0, 0.6);
            step(&mut fixture, 1);
            impact(&fixture, 2, 0.1, 0.2);
            step(&mut fixture, 2);
        }
        let field = fixture.read();
        let ends = field.endpoint.endpoints.map(|end| end.fine.unwrap());
        assert_eq!(ends[1].tick - ends[0].tick, 1);
        Some((
            field.fine().to_vec(),
            field.cells[ends[0].offset..ends[0].offset + SIDE * SIDE].to_vec(),
        ))
    };
    let (Some((separate, earlier)), Some((catch_up, intermediate))) = (run(false), run(true))
    else {
        return;
    };
    assert!(separate.iter().any(|state| state[0].abs() > 0.000_1));
    assert_eq!(
        separate, catch_up,
        "last two source ticks must advance separately and in order"
    );
    assert_eq!(
        earlier, intermediate,
        "the intermediate endpoint must hold the first substep"
    );
}

#[test]
fn tick_query_includes_the_submitted_fine_height() {
    let Some(mut fixture) = Fixture::new_fine(terrain(|_| -0.03)) else {
        return;
    };
    step(&mut fixture, 0);
    impact(&fixture, 1, 0.0, 0.6);
    step(&mut fixture, 1);

    let point = [0.127, 0.013];
    let request = fixture.endpoint(0);
    let use_tick = fixture.endpoint(1);
    fixture.query().post(
        request,
        vec![BodyQuery {
            id: 91,
            generation: 1,
            points: vec![QueryPoint {
                position: point,
                radius: 0.0,
                use_tick: 1,
            }],
        }],
    );
    fixture.ask_query();
    let mut field = fixture.read();
    let reply = fixture.query().drain(use_tick, Fixture::STEP);
    let height = reply[0].points[0].surface.sample.height;
    let expected = f64::from(field.height(point));
    field.endpoint.endpoints[1].fine = None;
    let base = f64::from(field.height(point));
    assert!(
        (base - expected).abs() > 0.000_2,
        "fine contribution {expected} vs base {base}"
    );
    assert!(
        (height - expected).abs() < 0.000_02,
        "{height} != {expected}"
    );
}

fn divided_puddles(offset: f64, spacing: f64, depth: f64) -> Map {
    let size = [100, 60];
    let base = [offset - 50.0 * spacing, -30.0 * spacing];
    let mut bed = Vec::new();
    let mut labels = Vec::new();
    for _ in 0..size[1] {
        for x in 0..size[0] {
            let sill = (47..53).contains(&x);
            bed.push(if sill { 0.008 } else { -depth });
            labels.push(u32::from(x >= 50));
        }
    }
    let hollows =
        Hollows::new(base, spacing, size, bed, labels, vec![0.0; 2]).expect("two puddles");
    Map::prepare(
        Bed::new([-4.0; 2], 1.0, [9, 9], vec![1.0; 81]).expect("dry ground"),
        vec![Body {
            name: "Puddles".into(),
            kind: BodyKind::Hollows(hollows),
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 1.0,
            gains: [0.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 0.5,
            swash_ratio: 0.7,
        },
    )
    .expect("prepared sill")
}

fn queried_surfaces(fixture: &mut Fixture, tick: u64, points: &[[f32; 2]]) -> Vec<Surface> {
    fixture.query().post(
        fixture.endpoint(tick - 1),
        vec![BodyQuery {
            id: 91,
            generation: 1,
            points: points
                .iter()
                .map(|&position| QueryPoint {
                    position,
                    radius: 0.0,
                    use_tick: tick,
                })
                .collect(),
        }],
    );
    fixture.ask_query();
    let replies = fixture.query().drain(fixture.endpoint(tick), Fixture::STEP);
    replies[0]
        .points
        .iter()
        .map(|reply| {
            assert!(reply.surface.wet);
            reply.surface
        })
        .collect()
}

#[test]
fn disconnected_puddles_stay_at_rest_on_every_level_and_in_the_query() {
    for shift in [0.0_f32, 0.125] {
        let offset = f64::from(shift);
        let Some(mut fixture) = Fixture::new_fine(divided_puddles(offset, 0.025, 0.10)) else {
            return;
        };
        step(&mut fixture, 0);
        impact(&fixture, 1, -0.22, 0.6);
        let points = [[-0.22, 0.0], [shift + 0.076, 0.0], [0.5, 0.0]];
        for tick in 1..=4 {
            step(&mut fixture, tick);
            let surfaces = queried_surfaces(&mut fixture, tick, &points);
            for surface in &surfaces[1..] {
                assert_eq!(
                    surface.sample,
                    Sample::default(),
                    "dry sill at {offset}, tick {tick}"
                );
            }
            let field = fixture.read();
            assert_eq!(field.height(points[2]), 0.0);
            let endpoint = field.endpoint.endpoints[1];
            let fine = endpoint.fine.unwrap();
            for (origin, metres, states) in [
                (endpoint.origins[0], Witness::CELL_METRES[0], field.level(0)),
                (endpoint.origins[1], Witness::CELL_METRES[1], field.level(1)),
                (fine.origin, Witness::FINE_CELL_METRES, field.fine()),
            ] {
                assert!(
                    states.iter().enumerate().all(|(i, &state)| {
                        let x = f64::from(origin[0]) + (i % SIDE) as f64 * f64::from(metres);
                        x < offset + 0.075 || state.iter().all(|v| v.abs().to_bits() == 0)
                    }),
                    "dry sill at {offset}, tick {tick}, spacing {metres}"
                );
            }
        }
        assert!(fixture.read().height(points[0]).abs() > 1e-4);
        fixture.set_levels(&[0.01, 0.01]);
        let mut reached = false;
        for tick in 5..=40 {
            step(&mut fixture, tick);
            if tick % 3 == 0 {
                reached |= (queried_surfaces(&mut fixture, tick, &points)[2]
                    .sample
                    .height
                    - 0.01)
                    .abs()
                    > 1e-5;
            }
        }
        assert!(
            reached,
            "the original source never reached the receiving puddle after flooding at {offset}"
        );
        eprintln!(
            "sill at {offset} m: exact rest in both state lanes and the public query until flooding, then transmission from the same source"
        );
    }
}

#[test]
fn fine_regions_keep_state_when_the_sill_floods() {
    let Some(mut fixture) = Fixture::new_fine(divided_puddles(0.0, CELL, 0.02)) else {
        return;
    };
    step(&mut fixture, 0);
    impact(&fixture, 1, -0.22, 0.6);
    for tick in 1..=9 {
        step(&mut fixture, tick);
    }
    let field = fixture.read();
    let end = field.endpoint.endpoints[1].fine.unwrap();
    let right = |index: usize| f64::from(end.origin[0]) + (index % SIDE) as f64 * CELL > 0.064;
    let at_rest = |state: [f32; 2]| state.into_iter().all(|v| v.abs().to_bits() == 0);
    assert!(
        field
            .fine()
            .iter()
            .enumerate()
            .all(|(i, &state)| !right(i) || at_rest(state))
    );
    assert!(!field.fine().iter().copied().all(at_rest));

    // Add a source on the other side just before the merge. Both inherited fields
    // must survive, and the same submitted planes must become one labelled region.
    impact(&fixture, 10, 0.22, 0.4);
    step(&mut fixture, 10);
    let before = fixture.read();
    fixture.set_levels(&[0.01, 0.01]);
    step(&mut fixture, 11);
    let after = fixture.read();
    let band = |field: &[[f32; 2]], side| {
        field
            .iter()
            .enumerate()
            .filter(|(i, _)| right(*i) == side)
            .fold([0.0_f64; 2], |mut sum, (_, v)| {
                for (total, value) in sum.iter_mut().zip(v) {
                    *total += f64::from(*value).powi(2);
                }
                sum
            })
    };
    for side in [false, true] {
        for (old, new) in band(before.fine(), side)
            .into_iter()
            .zip(band(after.fine(), side))
        {
            assert!(
                old > 1e-10 && new > old * 0.35,
                "live state cleared at a merge: {old} to {new}"
            );
        }
    }
    let sill = |i: usize| (f64::from(end.origin[0]) + (i % SIDE) as f64 * CELL).abs() < 0.045;
    assert!(
        before
            .fine()
            .iter()
            .enumerate()
            .all(|(i, &v)| !sill(i) || at_rest(v))
    );
    assert!(
        after
            .fine()
            .iter()
            .enumerate()
            .any(|(i, &v)| sill(i) && v[0].abs() > 1e-8)
    );
    assert_eq!(
        fixture
            .report
            .lock()
            .expect("report")
            .interaction
            .unwrap()
            .fine
            .unwrap()
            .regions,
        1
    );
    eprintln!(
        "isolated at exact rest through tick 9, both fields retained at tick 11 and height propagated across the inundated sill"
    );
}

#[test]
fn footfall_front_is_faster_on_deeper_water() {
    let run = |depth: f64| {
        let mut fixture = Fixture::new_fine(terrain(|_| -depth))?;
        step(&mut fixture, 0);
        impact(&fixture, 1, 0.0, 0.6);
        let mut arrivals = Vec::new();
        let mut energy = Vec::new();
        for tick in (1..=60).step_by(2) {
            step(&mut fixture, tick);
            let field = fixture.read();
            let end = field.endpoint.endpoints[1].fine.unwrap();
            let mut ring = 0.0_f64;
            for (i, value) in field.fine().iter().enumerate() {
                let x = f64::from(end.origin[0]) + (i % SIDE) as f64 * CELL;
                let z = f64::from(end.origin[1]) + (i / SIDE) as f64 * CELL;
                if (0.40..=0.46).contains(&x.hypot(z)) {
                    ring += f64::from(value[0]).powi(2);
                }
            }
            arrivals.push((tick, ring));
            // For a released source, the short packet's overall energy can
            // only fall. Sum the modes' spectral Hamiltonian by Parseval.
            let mut total = 0.0;
            for bin in [8, 16, 24] {
                let k = TAU * bin as f64 / (SIDE as f64 * CELL);
                let d = k * (k * depth).tanh();
                let restoring = 9.81 + CAPILLARY_COEFFICIENT * k * k;
                total += restoring * mode(field.fine(), bin, 0).powi(2)
                    + d * mode(field.fine(), bin, 1).powi(2);
            }
            energy.push(total);
        }
        let peak = arrivals.iter().map(|(_, e)| *e).fold(0.0, f64::max);
        let arrival = arrivals
            .into_iter()
            .find(|(_, e)| *e > peak * 0.5)
            .unwrap()
            .0;
        assert!(peak > 1e-8);
        // The source-free step uses blended depth operators. Rounding and
        // the finite border admit a small diagnostic tolerance.
        assert!(
            energy.windows(2).all(|pair| pair[1] <= pair[0] * 1.004),
            "energy at {depth} m: {energy:?}"
        );
        Some(arrival)
    };
    let (Some(shallow), Some(deep)) = (run(0.01), run(0.03)) else {
        return;
    };
    eprintln!("footfall arrival at 0.40–0.46 m: 10 mm tick {shallow}, 30 mm tick {deep}");
    assert!(deep + 4 <= shallow);
}
