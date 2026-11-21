//! The band is tested on the pixels of a unique overlap for every live pair.

#[expect(
    dead_code,
    reason = "each sprite proof uses a subset of the shared host fixture"
)]
mod support {
    pub(crate) mod sprite_fixture;
}
use support::sprite_fixture as fixture;

use thrum_render::{
    sprite::{SpriteRect, SpriteRow, SpriteScene},
    synthetic::CaptureSource,
};

const COUNT: usize = 8;
const PAIRS: usize = COUNT * (COUNT - 1) / 2;
const EXTENT: [u32; 2] = [64, 8];
const BAND: f64 = 1.0;

#[derive(Clone, Copy, Debug)]
struct Entity {
    generation: u64,
    identity: u128,
    sort: f64,
}

fn pair_slots() -> impl Iterator<Item = (usize, usize)> {
    (0..COUNT).flat_map(|left| (left + 1..COUNT).map(move |right| (left, right)))
}

fn code(slot: usize) -> u8 {
    32 + u8::try_from(slot).expect("eight physical slots") * 25
}

fn scene_config() -> SpriteScene {
    let mut pixels = Vec::new();
    for slot in 0..COUNT {
        for (left, right) in pair_slots() {
            pixels.extend(if slot == left || slot == right {
                [code(slot), 0, 0, 255]
            } else {
                [0; 4]
            });
        }
    }
    let frame_width = u32::try_from(PAIRS).expect("test atlas width");
    let frames: Vec<_> = (0_u32..8)
        .map(|slot| SpriteRect {
            origin: [slot * frame_width, 0],
            size: [frame_width, 1],
        })
        .collect();
    let mut scene = SpriteScene::new(fixture::sheet([frame_width * 8, 1], &pixels, &frames), 0);
    scene.world_capacity = 8;
    scene.order_band = BAND;
    scene
}

fn read_order(
    host: &mut fixture::Host,
    device: &wgpu::Device,
    entities: &[Option<Entity>; COUNT],
) -> Vec<usize> {
    let mut rows = Vec::new();
    for (slot, entity) in (0_u32..).zip(entities) {
        if let Some(entity) = entity {
            rows.push((
                slot,
                entity.generation,
                entity.identity,
                SpriteRow {
                    sort: Some(entity.sort),
                    ..SpriteRow::new(slot, [-27.0, 2.0])
                },
            ));
        }
    }
    rows.reverse();
    host.continuous_source_mut().world_rows = rows;
    let image = fixture::capture(host, device, EXTENT, CaptureSource::Sdr);
    let mut rank = [0; COUNT];
    for (index, (left, right)) in pair_slots().enumerate() {
        let address = (2 * EXTENT[0] as usize + 5 + index) * 4;
        let pixel = image.rgba[address];
        match (entities[left], entities[right]) {
            (Some(_), Some(_)) if pixel == code(left) => rank[left] += 1,
            (Some(_), Some(_)) if pixel == code(right) => rank[right] += 1,
            (Some(_), None) if pixel == code(left) => {}
            (None, Some(_)) if pixel == code(right) => {}
            (None, None) if pixel == 0 => {}
            _ => panic!("overlap between {left} and {right} produced red code {pixel}"),
        }
    }
    let mut actual: Vec<_> = (0..COUNT)
        .filter(|&slot| entities[slot].is_some())
        .collect();
    actual.sort_unstable_by_key(|&slot| rank[slot]);
    for (index, &slot) in actual.iter().enumerate() {
        assert_eq!(rank[slot], index, "cyclic overlap order");
    }
    actual
}

fn entity(slot: usize, sort: f64) -> Entity {
    Entity {
        generation: 1,
        identity: slot as u128,
        sort,
    }
}

fn equal_history(old: Option<Entity>, new: Option<Entity>) -> bool {
    match (old, new) {
        (Some(old), Some(new)) => (old.generation, old.identity) == (new.generation, new.identity),
        _ => false,
    }
}

fn pair_rule(
    previous: &[usize],
    current: &[usize],
    before: &[Option<Entity>; COUNT],
    now: &[Option<Entity>; COUNT],
) -> bool {
    let preserves = |slot| equal_history(before[slot], now[slot]);
    for (index, &left) in current
        .iter()
        .enumerate()
        .filter(|&(_, slot)| preserves(*slot))
    {
        for &right in current[index + 1..].iter().filter(|&&slot| preserves(slot)) {
            let old_left = previous
                .iter()
                .position(|&slot| slot == left)
                .expect("retained left");
            let old_right = previous
                .iter()
                .position(|&slot| slot == right)
                .expect("retained right");
            if old_right < old_left
                && now[right].expect("live").sort - now[left].expect("live").sort <= BAND
            {
                return false;
            }
        }
    }
    true
}

fn random(seed: &mut u64) -> u32 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    u32::try_from(*seed >> 32).expect("upper half")
}

#[test]
fn overlap_pixels_obey_the_pair_rule_with_travel_chains_and_entity_churn() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities: caps,
        ..
    }) = fixture::open("retained sprite order", "apple-tile")
    else {
        return;
    };
    let mut host = fixture::configured_host(
        &caps,
        &queue,
        EXTENT,
        fixture::Scene::new(Vec::new(), 1),
        scene_config(),
    );
    let mut state = [None; COUNT];
    let mut previous = [None; COUNT];
    let mut actual = Vec::new();
    let mut held = [0.0; COUNT];
    let mut previous_held = Vec::new();
    let mut held_broke = false;
    let mut seed = 4578;
    let mut generation = [1; COUNT];
    let mut reversals = 0;
    let mut insertions = 0;
    for tick in 0..384 {
        match tick {
            0..=3 => {
                let [a, b] = [[0.0, 0.1], [2.0, 2.1], [2.2, 2.1], [2.1, 2.3]][tick];
                state[0] = Some(entity(0, a));
                state[1] = Some(entity(1, b));
            }
            4 => {
                state[0] = Some(entity(0, -2.0));
                state[1] = Some(entity(1, -2.0));
                state[2] = Some(entity(2, 0.0));
            }
            5 => {
                // All three overlap. Each adjacent inversion is only 0.9,
                // although the endpoints of the chain differ by 1.8.
                for (slot, sort) in [1.8, 0.9, 0.0].into_iter().enumerate() {
                    state[slot] = Some(entity(slot, sort));
                }
            }
            6 => {
                state[0] = Some(entity(0, 4.0));
            }
            _ => {
                for (slot, entry) in state.iter_mut().enumerate() {
                    match (random(&mut seed) % 23, *entry) {
                        (0, Some(_)) => *entry = None,
                        (0..=5, _) | (_, None) => {
                            generation[slot] += 1;
                            *entry = Some(Entity {
                                generation: generation[slot],
                                identity: u128::from(generation[slot]) * 8 + slot as u128,
                                sort: f64::from(random(&mut seed) % 30) / 15.0,
                            });
                        }
                        (_, Some(mut entity)) => {
                            entity.sort += (f64::from(random(&mut seed) % 31) - 15.0) / 11.0;
                            *entry = Some(entity);
                        }
                    }
                }
            }
        }
        if tick == 2 {
            // A display callback can miss publications. The same live
            // identities still keep their overlap order on this timeline.
            host.world_mut().tick += 3;
        }
        let next = read_order(&mut host, &device, &state);
        if tick < 4 {
            assert_eq!(next, [0, 1], "travelling pair at tick {tick}");
        }
        if tick == 5 {
            assert_eq!(next, [0, 1, 2], "chain of three");
        }
        if tick == 6 {
            assert_eq!(next, [1, 2, 0], "real pass across the band");
        }
        assert!(
            pair_rule(&actual, &next, &previous, &state),
            "near reversal at tick {tick}"
        );
        for adjacent in next.windows(2) {
            let [left, right] = [adjacent[0], adjacent[1]];
            let a = state[left].expect("live");
            let b = state[right].expect("live");
            let exact = !equal_history(previous[left], state[left])
                || !equal_history(previous[right], state[right]);
            if exact {
                insertions += 1;
                assert!(
                    (a.sort, a.identity) <= (b.sort, b.identity),
                    "fresh neighbour at tick {tick}"
                );
            } else {
                assert!(
                    a.sort - b.sort <= BAND,
                    "unsettled neighbours at tick {tick}"
                );
            }
        }
        reversals += usize::from(tick > 0 && actual != next);
        let mut held_order: Vec<_> = next.clone();
        for &slot in &held_order {
            let value = state[slot].expect("live").sort;
            if !equal_history(previous[slot], state[slot]) || (value - held[slot]).abs() > BAND {
                held[slot] = value;
            }
        }
        held_order.sort_unstable_by(|&a, &b| {
            held[a].total_cmp(&held[b]).then_with(|| {
                state[a]
                    .expect("live")
                    .identity
                    .cmp(&state[b].expect("live").identity)
            })
        });
        held_broke |= !pair_rule(&previous_held, &held_order, &previous, &state);
        previous_held = held_order;
        previous = state;
        actual = next;
    }
    assert!(insertions > 300 && reversals > 250 && held_broke);
    println!(
        "384 submitted pair matrices, {reversals} ticks changed order, {insertions} fresh adjacent witnesses; held-value control violated the band"
    );
}
