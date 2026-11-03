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
use thrum_network::peer::{CaptureScratch, OrdinaryPeer, PeerPhase};
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
                && actor.owner
                    == if index == 0 {
                        u16::try_from(group_id).expect("bounded test value")
                    } else {
                        0
                    }
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
        world
            .spawn(StableId(id), Cell { value: 0 })
            .expect("test operation should succeed");
    }
    let mut authority = Authority::new(AUTHORITY, Rules, world);
    for peer in 1..=4 {
        authority
            .register_peer(PeerId(peer), peer + 100, controlled_actor(peer))
            .expect("test operation should succeed");
    }
    let baseline = authority
        .try_baseline()
        .expect("test operation should succeed");
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
    .expect("test operation should succeed")
}

fn queues() -> [UnsentQueue; 4] {
    let seeds: [_; 4] = std::array::from_fn(|index| PartitionSeed {
        group_id: u32::try_from(index).expect("bounded test value") + 1,
        interest_generation: 1,
        sample_tick: 0,
        resolved_through: 0,
    });
    std::array::from_fn(|index| {
        UnsentQueue::from_baseline(key(index as u64 + 1), &seeds)
            .expect("test operation should succeed")
    })
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
        .expect("test operation should succeed");
    assert!(matches!(admitted, thrum_core::Admission::Queued));
}

fn projected_actors(
    publication: &thrum_core::TickPublication<Cell, (), Rejection>,
) -> [[Actor; 32]; 4] {
    assert_eq!(publication.group.authority, AUTHORITY);
    assert_eq!(publication.snapshot.tick(), publication.group.commit_tick);
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
        let id = u64::try_from(record.id.0).expect("test operation should succeed");
        let partition = usize::try_from((id - 1) / 32).expect("test operation should succeed");
        let offset = counts[partition];
        assert!(offset < 32);
        actors[partition][offset].wire_id = id;
        actors[partition][offset].health = record.state.value;
        if (id - 1) % 32 == 0 {
            actors[partition][offset].owner =
                u16::try_from(partition + 1).expect("four partitions");
        }
        counts[partition] += 1;
    }
    assert_eq!(counts, [32; 4]);
    actors
}

fn result_for_peer(
    publication: &thrum_core::TickPublication<Cell, (), Rejection>,
    peer: PeerId,
) -> (u64, Option<ActionResult>) {
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
    (watermark, outcome)
}

fn capture_publication(
    publication: &thrum_core::TickPublication<Cell, (), Rejection>,
    queues: &mut [UnsentQueue; 4],
) {
    let tick = publication.group.commit_tick;
    let actors = projected_actors(publication);
    for (peer_index, queue) in queues.iter_mut().enumerate() {
        let peer = PeerId(peer_index as u64 + 1);
        let (watermark, outcome) = result_for_peer(publication, peer);
        let mut payloads = vec![vec![0u8; r4::MAX_PAYLOAD_BYTES]; 4];
        let mut lengths = [0usize; 4];
        for (partition, partition_actors) in actors.iter().enumerate() {
            let controlled = partition == peer_index;
            let one = outcome.filter(|_| controlled);
            lengths[partition] = r4::encode(
                GroupInput {
                    header: GroupHeader {
                        group_id: u32::try_from(partition).expect("bounded test value") + 1,
                        interest_generation: 1,
                        commit_tick: tick,
                        resolved_through: if controlled { watermark } else { 0 },
                    },
                    actors: partition_actors,
                    lifecycle: &[],
                    results: one.as_slice(),
                    events: &[],
                },
                &mut payloads[partition],
            )
            .expect("test operation should succeed");
        }
        let batch: [&[u8]; 4] = std::array::from_fn(|index| &payloads[index][..lengths[index]]);
        queue
            .capture_batch(
                &batch,
                &PartitionSchema {
                    controlled_group: u32::try_from(peer.0).expect("four peers"),
                },
            )
            .expect("test operation should succeed");
    }
}

fn owned_peers() -> [OrdinaryPeer; 4] {
    let seeds: [_; 4] = std::array::from_fn(|index| PartitionSeed {
        group_id: u32::try_from(index).expect("bounded test value") + 1,
        interest_generation: 1,
        sample_tick: 0,
        resolved_through: 0,
    });
    std::array::from_fn(|index| {
        OrdinaryPeer::from_baseline(key(index as u64 + 1), &seeds, 0, 0)
            .expect("test operation should succeed")
    })
}

fn guarded_peer_projection(
    peers: &mut [OrdinaryPeer; 4],
    project: impl FnOnce(&mut [OrdinaryPeer; 4]),
) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| project(peers)));
    if let Err(payload) = outcome {
        for owner in peers {
            owner.quarantine_projection_fault();
        }
        std::panic::resume_unwind(payload);
    }
}

fn capture_publication_owned(
    publication: &thrum_core::TickPublication<Cell, (), Rejection>,
    peers: &mut [OrdinaryPeer; 4],
    scratch: &mut CaptureScratch,
) {
    guarded_peer_projection(peers, |peers| {
        let tick = publication.group.commit_tick;
        let actors = projected_actors(publication);
        for (peer_index, owner) in peers.iter_mut().enumerate() {
            let peer = PeerId(peer_index as u64 + 1);
            let (watermark, outcome) = result_for_peer(publication, peer);
            owner
                .capture_projected(
                    scratch,
                    |writer| {
                        for (partition, partition_actors) in actors.iter().enumerate() {
                            let controlled = partition == peer_index;
                            let one = outcome.filter(|_| controlled);
                            writer.push(GroupInput {
                                header: GroupHeader {
                                    group_id: u32::try_from(partition).expect("bounded test value")
                                        + 1,
                                    interest_generation: 1,
                                    commit_tick: tick,
                                    resolved_through: if controlled { watermark } else { 0 },
                                },
                                actors: partition_actors,
                                lifecycle: &[],
                                results: one.as_slice(),
                                events: &[],
                            })?;
                        }
                        Ok::<_, thrum_network::peer::ScratchError>(())
                    },
                    &PartitionSchema {
                        controlled_group: u32::try_from(peer.0).expect("four peers"),
                    },
                )
                .expect("test operation should succeed");
        }
    });
}

fn submit_and_write_owned(owner: &mut OrdinaryPeer, checksum: &mut u64) {
    while owner
        .submit_next(0)
        .expect("test operation should succeed")
        .is_some()
    {}
    while let Some(frame) = owner.next_write().expect("test operation should succeed") {
        for byte in frame.first.iter().chain(frame.second) {
            *checksum = checksum.wrapping_mul(0x100_0000_01b3) ^ u64::from(*byte);
        }
        owner
            .accept_written(
                frame.sequence,
                frame.first.len() + frame.second.len() - frame.accepted_bytes,
            )
            .expect("test operation should succeed");
    }
}

fn submit_and_write(
    queue: &mut UnsentQueue,
    ledger: &mut OrdinaryLedger,
    key: PeerKey,
    checksum: &mut u64,
) {
    while ledger
        .submit_next_from_queue(queue, key, 0)
        .expect("test operation should succeed")
        .is_some()
    {}
    while let Some(frame) = ledger.next_write().expect("test operation should succeed") {
        for byte in frame.first.iter().chain(frame.second) {
            *checksum = checksum.wrapping_mul(0x100_0000_01b3) ^ u64::from(*byte);
        }
        ledger
            .accept_written(
                frame.sequence,
                frame.first.len() + frame.second.len() - frame.accepted_bytes,
            )
            .expect("test operation should succeed");
    }
}

#[test]
fn real_w1_snapshots_feed_four_peer_queues_without_waiting_for_slow_ack() {
    let origin = Instant::now();
    let mut host = host(origin);
    let mut queues = queues();
    let mut ledgers: [_; 4] = std::array::from_fn(|index| {
        OrdinaryLedger::new(key(index as u64 + 1), 0, 0).expect("test operation should succeed")
    });
    let mut checksum = 0xcbf2_9ce4_8422_2325u64;
    for tick in 1..=9 {
        submit_input(&mut host, 1, tick, tick - 1);
        if tick == 2 {
            submit_input(&mut host, 4, 1, 1);
        }
        let report = host
            .on_callback(callback_at(origin, tick))
            .expect("test operation should succeed");
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
                submit_and_write(
                    &mut queues[index],
                    &mut ledgers[index],
                    peer_key,
                    &mut checksum,
                );
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
                        .expect("test operation should succeed");
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
            .expect("test operation should succeed")
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
            .expect("test operation should succeed")
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
    assert_eq!(
        host.authority()
            .try_baseline()
            .expect("test operation should succeed")
            .results
            .len(),
        1
    );
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
        .expect("test operation should succeed");
    assert_eq!(ledgers[3].result_retired_through(), 1);
    assert_eq!(ledgers[3].retained_frame_bytes(), 0);
    assert_eq!(ledgers[3].outstanding_entries(), 0);
    assert_eq!(host.authority().snapshot().tick(), 9);
    assert_eq!(
        host.authority()
            .try_baseline()
            .expect("test operation should succeed")
            .results,
        Vec::new()
    );
    assert_ne!(checksum, 0);
}

#[test]
fn real_w1_publications_feed_one_owned_peer_per_connection() {
    let origin = Instant::now();
    let mut host = host(origin);
    let mut peers = owned_peers();
    let mut scratch = CaptureScratch::new();
    let mut checksum = 0xcbf2_9ce4_8422_2325u64;
    for tick in 1..=9 {
        submit_input(&mut host, 1, tick, tick - 1);
        if tick == 2 {
            submit_input(&mut host, 4, 1, 1);
        }
        let report = host
            .on_callback(callback_at(origin, tick))
            .expect("test operation should succeed");
        assert_eq!(report.publications.len(), 1);
        let HostPublication::Local(publication) = &report.publications[0] else {
            panic!("authority-only host must publish full local group");
        };
        capture_publication_owned(publication, &mut peers, &mut scratch);
        drop(report);
        if tick % 3 == 0 {
            for (index, owner) in peers.iter_mut().enumerate() {
                submit_and_write_owned(owner, &mut checksum);
                if index != 3 {
                    owner
                        .acknowledge_through(
                            key(index as u64 + 1),
                            owner.last_sequence(),
                            |identity, through| {
                                host.acknowledge_results_for_incarnation(
                                    AUTHORITY,
                                    PeerId(index as u64 + 1),
                                    identity.connection_epoch,
                                    through,
                                )
                            },
                        )
                        .expect("test operation should succeed");
                }
            }
        }
    }
    assert_eq!(host.authority().snapshot().tick(), 9);
    for peer in 1..=4 {
        let id = StableId(u128::from(controlled_actor(peer).0));
        let value = host
            .authority()
            .snapshot()
            .records()
            .find(|record| record.id == id)
            .expect("test operation should succeed")
            .state
            .value;
        assert_eq!(value, if peer == 1 { 9 } else { u16::from(peer == 4) });
    }
    assert!(peers.iter().all(|peer| peer.queued_groups() == 0));
    assert!(peers[3].outstanding_frames() > 8);
    assert_eq!(peers[3].result_retired_through(), 0);
    assert_eq!(
        host.authority()
            .try_baseline()
            .expect("test operation should succeed")
            .results
            .len(),
        1
    );
    let through = peers[3].last_sequence();
    peers[3]
        .acknowledge_through(key(4), through, |identity, result_prefix| {
            host.acknowledge_results_for_incarnation(
                AUTHORITY,
                PeerId(4),
                identity.connection_epoch,
                result_prefix,
            )
        })
        .expect("test operation should succeed");
    assert_eq!(peers[3].result_retired_through(), 1);
    assert_eq!(peers[3].outstanding_frames(), 0);
    assert_eq!(
        host.authority()
            .try_baseline()
            .expect("test operation should succeed")
            .results,
        Vec::new()
    );
    assert_ne!(checksum, 0);
}

#[test]
fn caught_shared_extraction_unwind_closes_all_affected_peers() {
    let mut peers = owned_peers();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        guarded_peer_projection(&mut peers, |_| panic!("shared W1 extraction interrupted"));
    }));
    assert!(outcome.is_err());
    assert!(
        peers
            .iter()
            .all(|peer| peer.phase() == PeerPhase::RequiresClose)
    );
    assert!(peers.iter().all(|peer| peer.queued_groups() == 0));
    assert!(peers.iter().all(|peer| peer.last_sequence() == 0));
}
