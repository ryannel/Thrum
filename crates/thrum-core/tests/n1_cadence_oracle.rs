//! Selected inline model for the N1 60-to-20 Hz construction decision.
//! It is not a production multi-group replica or network codec.

use thrum_core::{
    ActionRejection, ActionResult, AuthorityId, CommittedEvent, EntityChange, EntityRecord,
    MatchId, Origin, PeerId, Replica, ResultDisposition, StableId, StateGroup, World, WorldEpoch,
};

type Group = StateGroup<u64, u64, ()>;
const AUTHORITY: AuthorityId = AuthorityId::Match(MatchId(900));
const CONTROLLER: StableId = StableId(1);
const STATIC: StableId = StableId(2);
const TRANSIENT: StableId = StableId(3);
const PEER: PeerId = PeerId(1);

#[derive(Clone)]
struct Unit {
    partition: usize,
    wire_id: u32,
    complete_state: bool,
    group: Group,
}

struct CadenceModel {
    pending: Vec<Unit>,
    next_wire_id: u32,
}

impl CadenceModel {
    fn new() -> Self {
        Self {
            pending: Vec::new(),
            next_wire_id: 1,
        }
    }

    fn stage(&mut self, partition: usize, group: Group, complete_state: bool) {
        if group.changes.is_empty()
            && group.results.is_empty()
            && group.events.is_empty()
            && !complete_state
        {
            return;
        }
        let replaceable = state_only(&group, complete_state);
        if replaceable
            && let Some(index) = self
                .pending
                .iter()
                .rposition(|unit| unit.partition == partition)
            && state_only(
                &self.pending[index].group,
                self.pending[index].complete_state,
            )
            && group.resolved_through == self.pending[index].group.resolved_through
            && group.event_cursor == self.pending[index].group.event_cursor
        {
            self.pending[index].group = group;
            return;
        }
        self.pending.push(Unit {
            partition,
            wire_id: 0,
            complete_state,
            group,
        });
    }

    fn stage_boundary_sample(&mut self, partition: usize, sample: Group) {
        assert!(state_only(&sample, true));
        if let Some(index) = self.pending.iter().rposition(|unit| {
            unit.partition == partition && unit.group.commit_tick == sample.commit_tick
        }) {
            let existing = &mut self.pending[index];
            assert_eq!(existing.group.resolved_through, sample.resolved_through);
            assert_eq!(existing.group.event_cursor, sample.event_cursor);
            if state_only(&existing.group, existing.complete_state) {
                existing.group = sample;
            } else {
                for change in sample.changes {
                    if let Some(prior) = existing
                        .group
                        .changes
                        .iter()
                        .find(|prior| change_id(prior) == change_id(&change))
                    {
                        match (prior, &change) {
                            (
                                EntityChange::Spawn(a) | EntityChange::Update(a),
                                EntityChange::Update(b),
                            ) if a == b => {}
                            _ => panic!("boundary sample conflicts with mandatory state"),
                        }
                    } else {
                        existing.group.changes.push(change);
                    }
                }
            }
            existing.complete_state = true;
        } else {
            self.stage(partition, sample, true);
        }
    }

    fn release(&mut self) -> Vec<Unit> {
        let mut released = std::mem::take(&mut self.pending);
        released.sort_by_key(|unit| (unit.group.commit_tick, unit.partition));
        for unit in &mut released {
            unit.wire_id = self.next_wire_id;
            self.next_wire_id += 1;
        }
        released
    }
}

fn state_only(group: &Group, complete_state: bool) -> bool {
    complete_state
        && group.results.is_empty()
        && group.events.is_empty()
        && group
            .changes
            .iter()
            .all(|change| matches!(change, EntityChange::Update(_)))
}

fn change_id(change: &EntityChange<u64>) -> StableId {
    match change {
        EntityChange::Spawn(record) | EntityChange::Update(record) => record.id,
        EntityChange::Despawn(id) => *id,
    }
}

fn group(tick: u64, changes: Vec<EntityChange<u64>>, watermark: u64) -> Group {
    Group {
        authority: AUTHORITY,
        group_id: 0, // The model assigns wire IDs only when units are released.
        commit_tick: tick,
        event_cursor: 0,
        changes,
        results: Vec::new(),
        events: Vec::new(),
        resolved_through: vec![(PEER, watermark)],
    }
}

fn baseline(id: StableId, watermark: u64) -> Group {
    group(
        0,
        vec![EntityChange::Spawn(EntityRecord { id, state: 0 })],
        watermark,
    )
}

fn next_result(
    tick: u64,
    sequence: u64,
    disposition: ResultDisposition<()>,
    event_id: Option<u64>,
) -> ActionResult<()> {
    ActionResult {
        origin: Origin {
            peer: PEER,
            connection_epoch: 1,
            sequence,
        },
        resolved_tick: tick,
        disposition,
        event_id,
    }
}

#[test]
fn mandatory_tick_units_survive_three_tick_release_and_same_tick_partitions() {
    let mut world = World::new(WorldEpoch(900));
    let controller = world.spawn(CONTROLLER, 0).unwrap();
    world.spawn(STATIC, 0).unwrap();
    world.publish(0);
    let (mut controller_replica, _) = Replica::from_baseline(baseline(CONTROLLER, 6)).unwrap();
    let (mut static_replica, _) = Replica::from_baseline(Group {
        resolved_through: Vec::new(),
        ..baseline(STATIC, 0)
    })
    .unwrap();
    let mut per_partition_id = [0_u64; 2];
    let mut model = CadenceModel::new();
    let mut transient = None;
    let mut expected_controller = [0_u64; 7];

    for tick in 1..=6 {
        match tick {
            1 => {
                *world.get_mut(controller).unwrap() = 1;
                transient = Some(world.spawn(TRANSIENT, 7).unwrap());
            }
            2 => {
                *world.get_mut(controller).unwrap() = 2;
                *world.get_mut(transient.unwrap()).unwrap() = 8;
            }
            3 => {
                *world.get_mut(controller).unwrap() = 3;
                world.despawn(transient.unwrap()).unwrap();
            }
            _ => {}
        }
        let publication = world.publish(tick);
        expected_controller[tick as usize] = publication.snapshot.get(controller).unwrap().state;
        let mut left = group(
            tick,
            publication
                .changes
                .iter()
                .filter(|change| change_id(change) == CONTROLLER)
                .cloned()
                .collect(),
            if tick >= 5 { 8 } else { 6 },
        );
        let mut right = group(
            tick,
            publication
                .changes
                .iter()
                .filter(|change| change_id(change) != CONTROLLER)
                .cloned()
                .collect(),
            0,
        );
        right.resolved_through.clear();
        if tick == 4 {
            // A rejection has no W1 change, but its mandatory result must
            // carry the unchanged complete controller at this exact tick.
            left.changes.push(EntityChange::Update(EntityRecord {
                id: CONTROLLER,
                state: 3,
            }));
            left.results.push(next_result(
                4,
                8,
                ResultDisposition::Rejected(ActionRejection::Game(())),
                None,
            ));
        }
        if tick == 5 {
            left.changes.push(EntityChange::Update(EntityRecord {
                id: CONTROLLER,
                state: 3,
            }));
            left.results
                .push(next_result(5, 7, ResultDisposition::Committed, Some(1)));
            left.events.push(CommittedEvent {
                id: 1,
                tick: 5,
                origin: Origin {
                    peer: PEER,
                    connection_epoch: 1,
                    sequence: 7,
                },
                payload: 99,
            });
            left.event_cursor = 1;
        }
        if tick == 6 {
            left.event_cursor = 1;
        }
        model.stage(0, left, tick <= 3);
        model.stage(1, right, false);
        if tick % 3 == 0 {
            // Sample the complete current partition state. At tick 3 the
            // right partition already has a mandatory despawn unit, so the
            // sample must merge into it rather than add a fourth unit.
            let mut left_sample = group(
                tick,
                publication
                    .snapshot
                    .records()
                    .filter(|record| record.id == CONTROLLER)
                    .cloned()
                    .map(EntityChange::Update)
                    .collect(),
                if tick >= 5 { 8 } else { 6 },
            );
            left_sample.event_cursor = u64::from(tick >= 5);
            let mut right_sample = group(
                tick,
                publication
                    .snapshot
                    .records()
                    .filter(|record| record.id != CONTROLLER)
                    .cloned()
                    .map(EntityChange::Update)
                    .collect(),
                0,
            );
            right_sample.resolved_through.clear();
            model.stage_boundary_sample(0, left_sample);
            model.stage_boundary_sample(1, right_sample);
        }
        drop(publication);

        if tick % 3 != 0 {
            continue;
        }
        let batch = model.release();
        assert!(batch.iter().all(|unit| unit.group.commit_tick <= tick));
        assert!(batch.iter().all(|unit| unit.group.commit_tick > tick - 3));
        for unit in &batch {
            if !unit.group.results.is_empty() {
                let controllers: Vec<_> = unit
                    .group
                    .changes
                    .iter()
                    .filter_map(|change| match change {
                        EntityChange::Update(record) if record.id == CONTROLLER => {
                            Some(record.state)
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    controllers,
                    [expected_controller[unit.group.commit_tick as usize]]
                );
            }
            per_partition_id[unit.partition] += 1;
            let mut local = unit.group.clone();
            local.group_id = per_partition_id[unit.partition];
            match unit.partition {
                0 => {
                    controller_replica.apply(local).unwrap();
                }
                1 => {
                    static_replica.apply(local).unwrap();
                }
                _ => unreachable!(),
            }
            if tick == 3 && unit.partition == 1 {
                match unit.group.commit_tick {
                    1 => assert_eq!(static_replica.get(TRANSIENT), Some(&7)),
                    2 => assert_eq!(static_replica.get(TRANSIENT), Some(&8)),
                    3 => assert_eq!(static_replica.get(TRANSIENT), None),
                    _ => unreachable!(),
                }
            }
        }
        if tick == 3 {
            assert_eq!(batch.len(), 4);
            let right_boundary: Vec<_> = batch
                .iter()
                .filter(|unit| unit.partition == 1 && unit.group.commit_tick == 3)
                .collect();
            assert_eq!(right_boundary.len(), 1);
            assert!(
                right_boundary[0]
                    .group
                    .changes
                    .iter()
                    .any(|change| { matches!(change, EntityChange::Despawn(TRANSIENT)) })
            );
            assert!(right_boundary[0].group.changes.iter().any(|change| {
                matches!(
                    change,
                    EntityChange::Update(EntityRecord {
                        id: STATIC,
                        state: 0
                    })
                )
            }));
            assert_eq!(
                batch.iter().map(|unit| unit.wire_id).collect::<Vec<_>>(),
                vec![1, 2, 3, 4]
            );
            assert_eq!(
                batch
                    .iter()
                    .filter(|unit| unit.group.commit_tick == 3)
                    .count(),
                2
            );
            assert_eq!(controller_replica.get(CONTROLLER), Some(&3));
        } else {
            assert_eq!(batch.iter().filter(|unit| unit.partition == 0).count(), 3);
            assert_eq!(controller_replica.get(CONTROLLER), Some(&3));
            assert_eq!(static_replica.get(STATIC), Some(&0));
            assert_eq!(batch.last().unwrap().wire_id, 8);
        }
    }
}

#[test]
fn partial_updates_of_different_entities_do_not_coalesce() {
    let mut model = CadenceModel::new();
    model.stage(
        0,
        group(
            1,
            vec![EntityChange::Update(EntityRecord {
                id: CONTROLLER,
                state: 1,
            })],
            0,
        ),
        false,
    );
    model.stage(
        0,
        group(
            2,
            vec![EntityChange::Update(EntityRecord {
                id: STATIC,
                state: 2,
            })],
            0,
        ),
        false,
    );
    let released = model.release();
    assert_eq!(released.len(), 2);
    assert_eq!(released[0].group.commit_tick, 1);
    assert_eq!(released[1].group.commit_tick, 2);
}
