//! Actual headless W1 publication copied into four peer-owned R4 queues.
//! The fixture is no-socket and does not claim a general game projection API.

use std::time::{Duration, Instant};

use thrum_core::{
    ActorId, AdmissionError, Authority, AuthorityId, ClockConfig, GameRules, HeadlessHost,
    HostBudget, HostError, HostPublication, Input, InputSchedule, MatchId, Origin, PeerId,
    PreparedCommit, PreparedEdits, ResultDisposition, RuleContext, StableId, World, WorldEdit,
    WorldEpoch,
};
use thrum_network::outbound::{OutboundValidator, PartitionSeed, StateCoverage, UnsentQueue};
use thrum_network::r4::{self, ActionResult, Actor, GroupHeader, GroupInput};
use thrum_network::retirement::PeerKey;
use thrum_network::sent::OrdinaryLedger;

const MATCH: MatchId = MatchId(42);
const AUTHORITY: AuthorityId = AuthorityId::Match(MATCH);

#[derive(Clone, Debug, Eq, PartialEq)]
struct Cell {
    value: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Increment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Rejection {
    UnknownActor,
    Overflow,
}

struct Rules;
impl GameRules for Rules {
    type Entity = Cell;
    type Action = Action;
    type Change = (StableId, u16);
    type Rejection = Rejection;
    type Event = ();

    fn decide(
        &self,
        world: &World<Cell>,
        actor: ActorId,
        _action: &Action,
        _context: &mut RuleContext<'_>,
    ) -> Result<Self::Change, Rejection> {
        let id = StableId(u128::from(actor.0));
        let handle = world.handle_for(id).ok_or(Rejection::UnknownActor)?;
        let state = &world.get(handle).ok_or(Rejection::UnknownActor)?.state;
        let next = state.value.checked_add(1).ok_or(Rejection::Overflow)?;
        Ok((id, next))
    }

    fn prepare(
        &self,
        world: &World<Cell>,
        (id, value): Self::Change,
        _context: &mut RuleContext<'_>,
    ) -> PreparedCommit<Cell, ()> {
        let handle = world.handle_for(id).expect("decided live object");
        PreparedCommit {
            edits: PreparedEdits::One(WorldEdit::Replace(handle, Cell { value })),
            event: None,
        }
    }
}

struct PartitionSchema {
    controlled_group: u32,
}
impl OutboundValidator for PartitionSchema {
    fn group(&self, group: &r4::GroupView<'_>) -> Option<StateCoverage> {
        let group_id = group.header.group_id;
        if !(1..=4).contains(&group_id) || group.actors().len() != 32 || group.events().len() != 0 {
            return None;
        }
        let first = u64::from(group_id - 1) * 32 + 1;
        if !group.actors().enumerate().all(|(index, actor)| {
            actor.wire_id == first + index as u64
                && actor.mode == 7
                && actor.owner == if index == 0 { group_id as u16 } else { 0 }
                && actor.visibility_generation == 1
                && actor.quaternion == [0, 0, 0, 32767]
        }) {
            return None;
        }
        if group_id != self.controlled_group
            && (group.results().len() != 0 || group.header.resolved_through != 0)
        {
            return None;
        }
        if group.results().any(|result| {
            result.action_type != 1
                || result.disposition != 0
                || result.reason != 0
                || result.event_count != 0
        }) {
            return None;
        }
        Some(StateCoverage::CompletePartition)
    }
}

fn key(peer: u64) -> PeerKey {
    PeerKey {
        match_id: MATCH.0.to_le_bytes(),
        peer_id: peer,
        connection_epoch: peer + 100,
    }
}

fn controlled_actor(peer: u64) -> ActorId {
    ActorId((peer - 1) * 32 + 1)
}

fn callback_at(origin: Instant, tick: u64) -> Instant {
    // Round upward so each synthetic 60 Hz callback has exactly one due tick.
    origin + Duration::from_nanos((tick * 1_000_000_000).div_ceil(60))
}

fn host(origin: Instant) -> HeadlessHost<Rules> {
    let mut world = World::new(WorldEpoch(1));
    for id in 1..=128 {
        world.spawn(StableId(id), Cell { value: 0 }).unwrap();
    }
    let mut authority = Authority::new(AUTHORITY, Rules, world);
    for peer in 1..=4 {
        authority
            .register_peer(PeerId(peer), peer + 100, controlled_actor(peer))
            .unwrap();
    }
    let baseline = authority.try_baseline().unwrap();
    assert_eq!(baseline.commit_tick, 0);
    assert_eq!(baseline.changes.len(), 128);
    drop(baseline);
    HeadlessHost::local_with_peer_limit(
        origin,
        ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(100),
            other_work: Duration::from_millis(1),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
        4,
    )
    .unwrap()
}

fn queues() -> [UnsentQueue; 4] {
    let seeds: [_; 4] = std::array::from_fn(|index| PartitionSeed {
        group_id: index as u32 + 1,
        interest_generation: 1,
        sample_tick: 0,
        resolved_through: 0,
    });
    std::array::from_fn(|index| UnsentQueue::from_baseline(key(index as u64 + 1), &seeds).unwrap())
}

fn submit_input(host: &mut HeadlessHost<Rules>, peer: u64, sequence: u64, observed_tick: u64) {
    let admitted = host
        .submit(Input {
            origin: Origin {
                peer: PeerId(peer),
                connection_epoch: peer + 100,
                sequence,
            },
            actor: controlled_actor(peer),
            schedule: InputSchedule::NextTick { observed_tick },
            action: Action::Increment,
        })
        .unwrap();
    assert!(matches!(admitted, thrum_core::Admission::Queued));
}

fn capture_publication(
    publication: &thrum_core::TickPublication<Cell, (), Rejection>,
    queues: &mut [UnsentQueue; 4],
) {
    assert_eq!(publication.group.authority, AUTHORITY);
    assert_eq!(publication.snapshot.tick(), publication.group.commit_tick);
    let tick = publication.group.commit_tick;
    let mut actors = [[Actor {
        wire_id: 0,
        sector: [0; 3],
        offset: [0; 3],
        quaternion: [0, 0, 0, 32767],
        velocity: [0; 3],
        mode: 7,
        flags: 0,
        health: 0,
        inventory_revision: 0,
        owner: 0,
        visibility_generation: 1,
    }; 32]; 4];
    let mut counts = [0usize; 4];
    for record in publication.snapshot.records() {
        let id = u64::try_from(record.id.0).unwrap();
        let partition = usize::try_from((id - 1) / 32).unwrap();
        let offset = counts[partition];
        assert!(offset < 32);
        actors[partition][offset].wire_id = id;
        actors[partition][offset].health = record.state.value;
        if (id - 1) % 32 == 0 {
            actors[partition][offset].owner = (partition + 1) as u16;
        }
        counts[partition] += 1;
    }
    assert_eq!(counts, [32; 4]);
    for (peer_index, queue) in queues.iter_mut().enumerate() {
        let peer = PeerId(peer_index as u64 + 1);
        let watermark = publication
            .group
            .resolved_through
            .iter()
            .find_map(|&(id, through)| (id == peer).then_some(through))
            .unwrap_or(0);
        let mut outcome = None;
        for result in &publication.group.results {
            if result.origin.peer == peer {
                assert!(outcome.is_none());
                assert!(matches!(result.disposition, ResultDisposition::Committed));
                assert!(result.event_id.is_none());
                outcome = Some(ActionResult {
                    input_sequence: result.origin.sequence,
                    resolved_tick: result.resolved_tick,
                    first_event_id: 0,
                    action_type: 1,
                    reason: 0,
                    event_count: 0,
                    disposition: 0,
                });
            }
        }
        for (partition, partition_actors) in actors.iter().enumerate() {
            let controlled = partition == peer_index;
            let one = outcome.filter(|_| controlled);
            let mut payload = [0u8; r4::MAX_PAYLOAD_BYTES];
            let len = r4::encode(
                GroupInput {
                    header: GroupHeader {
                        group_id: partition as u32 + 1,
                        interest_generation: 1,
                        commit_tick: tick,
                        resolved_through: if controlled { watermark } else { 0 },
                    },
                    actors: partition_actors,
                    lifecycle: &[],
                    results: one.as_slice(),
                    events: &[],
                },
                &mut payload,
            )
            .unwrap();
            queue
                .capture(
                    &payload[..len],
                    &PartitionSchema {
                        controlled_group: peer.0 as u32,
                    },
                )
                .unwrap();
        }
    }
}

fn submit_and_write(queue: &mut UnsentQueue, ledger: &mut OrdinaryLedger, key: PeerKey) {
    while ledger
        .submit_next_from_queue(queue, key, 0)
        .unwrap()
        .is_some()
    {}
    while let Some(frame) = ledger.next_write().unwrap() {
        ledger
            .accept_written(
                frame.sequence,
                frame.first.len() + frame.second.len() - frame.accepted_bytes,
            )
            .unwrap();
    }
}

#[test]
fn real_w1_snapshots_feed_four_peer_queues_without_waiting_for_slow_ack() {
    let origin = Instant::now();
    let mut host = host(origin);
    let mut queues = queues();
    let mut ledgers: [_; 4] =
        std::array::from_fn(|index| OrdinaryLedger::new(key(index as u64 + 1), 0, 0).unwrap());
    for tick in 1..=9 {
        submit_input(&mut host, 1, tick, tick - 1);
        if tick == 2 {
            submit_input(&mut host, 4, 1, 1);
        }
        let report = host.on_callback(callback_at(origin, tick)).unwrap();
        assert_eq!(report.publications.len(), 1);
        match &report.publications[0] {
            HostPublication::Local(publication) => {
                assert_eq!(publication.group.commit_tick, tick);
                capture_publication(publication, &mut queues);
            }
            _ => panic!("authority-only host must publish full local group"),
        }
        drop(report); // Never retain W1 pages while remote peers wait.
        if tick % 3 == 0 {
            for index in 0..4 {
                let peer_key = key(index as u64 + 1);
                submit_and_write(&mut queues[index], &mut ledgers[index], peer_key);
                if index != 3 {
                    let peer = PeerId(index as u64 + 1);
                    ledgers[index]
                        .acknowledge_through(
                            peer_key,
                            ledgers[index].last_sequence(),
                            |id, through| {
                                host.acknowledge_results_for_incarnation(
                                    AUTHORITY,
                                    peer,
                                    id.connection_epoch,
                                    through,
                                )
                            },
                        )
                        .unwrap();
                }
            }
        }
    }
    assert_eq!(host.authority().snapshot().tick(), 9);
    assert_eq!(
        host.authority()
            .snapshot()
            .records()
            .find(|record| record.id == StableId(1))
            .unwrap()
            .state
            .value,
        9
    );
    assert!(queues.iter().all(UnsentQueue::is_empty));
    assert!(ledgers[3].outstanding_entries() > 8);
    assert!(ledgers[3].retained_frame_bytes() > 0);
    assert_eq!(ledgers[3].result_retired_through(), 0);
    assert_eq!(
        host.authority()
            .try_baseline()
            .unwrap()
            .results
            .iter()
            .map(|result| result.origin)
            .collect::<Vec<_>>(),
        [Origin {
            peer: PeerId(4),
            connection_epoch: 104,
            sequence: 1,
        }]
    );
    assert_eq!(
        host.acknowledge_results_for_incarnation(AUTHORITY, PeerId(4), 103, 1),
        Err(HostError::Acknowledgement(AdmissionError::StaleEpoch))
    );
    assert_eq!(
        host.acknowledge_results_for_incarnation(
            AuthorityId::Match(MatchId(43)),
            PeerId(4),
            104,
            1,
        ),
        Err(HostError::Acknowledgement(AdmissionError::WrongAuthority))
    );
    assert_eq!(ledgers[3].result_retired_through(), 0);
    assert_eq!(host.authority().try_baseline().unwrap().results.len(), 1);
    let through = ledgers[3].last_sequence();
    ledgers[3]
        .acknowledge_through(key(4), through, |identity, result_prefix| {
            host.acknowledge_results_for_incarnation(
                AUTHORITY,
                PeerId(4),
                identity.connection_epoch,
                result_prefix,
            )
        })
        .unwrap();
    assert_eq!(ledgers[3].result_retired_through(), 1);
    assert_eq!(ledgers[3].retained_frame_bytes(), 0);
    assert_eq!(ledgers[3].outstanding_entries(), 0);
    assert_eq!(host.authority().snapshot().tick(), 9);
    assert!(host.authority().try_baseline().unwrap().results.is_empty());
}

#[test]
#[ignore = "local release-only real W1-to-N1 state-only path probe"]
fn measure_real_w1_to_n1_state_only_path() {
    measure_real_w1_to_n1_path::<false>("state_only");
}

#[test]
#[ignore = "local release-only real W1-to-N1 four-peer result-burst probe"]
fn measure_real_w1_to_n1_result_burst_path() {
    measure_real_w1_to_n1_path::<true>("result_burst");
}

fn measure_real_w1_to_n1_path<const RESULTS: bool>(kind: &str) {
    use std::hint::black_box;

    const WINDOWS: u64 = 2_000;
    const WARMUP_WINDOWS: u64 = 10;
    let origin = Instant::now();
    let mut host = host(origin);
    let mut queues = queues();
    let mut ledgers: [_; 4] =
        std::array::from_fn(|index| OrdinaryLedger::new(key(index as u64 + 1), 0, 0).unwrap());
    let reserved = queues
        .iter()
        .map(UnsentQueue::reserved_capacity_bytes)
        .sum::<usize>()
        + ledgers
            .iter()
            .map(OrdinaryLedger::reserved_capacity_bytes)
            .sum::<usize>();
    let mut checksum = 0xcbf2_9ce4_8422_2325u64;
    let mut written_frames = 0u64;
    let mut written_bytes = 0u64;
    let mut measured_start = None;
    for window in 0..WARMUP_WINDOWS + WINDOWS {
        if window == WARMUP_WINDOWS {
            checksum = 0xcbf2_9ce4_8422_2325;
            written_frames = 0;
            written_bytes = 0;
            measured_start = Some(Instant::now());
        }
        for within in 1..=3 {
            let tick = window * 3 + within;
            if RESULTS {
                for peer in 1..=4 {
                    submit_input(&mut host, peer, tick, tick - 1);
                }
            }
            let report = host.on_callback(callback_at(origin, tick)).unwrap();
            assert_eq!(report.publications.len(), 1);
            let HostPublication::Local(publication) = &report.publications[0] else {
                panic!("authority-only host must publish local group");
            };
            capture_publication(publication, &mut queues);
            drop(report);
        }
        for index in 0..4 {
            let peer_key = key(index as u64 + 1);
            while ledgers[index]
                .submit_next_from_queue(&mut queues[index], peer_key, 0)
                .unwrap()
                .is_some()
            {}
            while let Some(frame) = ledgers[index].next_write().unwrap() {
                written_frames += 1;
                written_bytes += (frame.first.len() + frame.second.len()) as u64;
                for byte in frame.first.iter().chain(frame.second) {
                    checksum = checksum.wrapping_mul(0x100_0000_01b3) ^ u64::from(*byte);
                }
                ledgers[index]
                    .accept_written(
                        frame.sequence,
                        frame.first.len() + frame.second.len() - frame.accepted_bytes,
                    )
                    .unwrap();
            }
            ledgers[index]
                .acknowledge_through(peer_key, ledgers[index].last_sequence(), |id, through| {
                    host.acknowledge_results_for_incarnation(
                        AUTHORITY,
                        PeerId(index as u64 + 1),
                        id.connection_epoch,
                        through,
                    )
                })
                .unwrap();
        }
    }
    let elapsed = measured_start.unwrap().elapsed();
    assert_eq!(
        host.authority().snapshot().tick(),
        (WARMUP_WINDOWS + WINDOWS) * 3
    );
    for peer in 1..=4 {
        let id = StableId(u128::from(controlled_actor(peer).0));
        let value = host
            .authority()
            .snapshot()
            .records()
            .find(|record| record.id == id)
            .unwrap()
            .state
            .value;
        assert_eq!(
            value,
            if RESULTS {
                (WARMUP_WINDOWS + WINDOWS) as u16 * 3
            } else {
                0
            }
        );
    }
    assert!(host.authority().try_baseline().unwrap().results.is_empty());
    assert!(queues.iter().all(UnsentQueue::is_empty));
    assert!(
        ledgers
            .iter()
            .all(|ledger| ledger.outstanding_entries() == 0)
    );
    assert_eq!(written_frames, WINDOWS * if RESULTS { 24 } else { 16 });
    assert_eq!(
        written_bytes,
        WINDOWS * if RESULTS { 45_696 } else { 30_208 }
    );
    black_box(checksum);
    println!(
        "w1_to_n1_{kind},windows={WINDOWS},ns_per_window={:.1},reserved_queue_ledger_bytes={reserved},frames={written_frames},bytes={written_bytes},checksum={checksum}",
        elapsed.as_nanos() as f64 / WINDOWS as f64,
    );
}
