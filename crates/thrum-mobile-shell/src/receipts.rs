//! Infallible delivery and acknowledgement for the one-peer local phone game.

use thrum_core::{
    GameHostCallback, GameRules, HeadlessHost, HostError, HostPublication, PeerId, WorldEpoch,
};

/// Delivery failure in the bounded local phone path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalReceiptError {
    /// A previous observer panic or acknowledgement error stopped this cursor.
    Poisoned,
    /// The callback carried a second peer and needs a multiplayer consumer.
    UnexpectedPeer,
    /// A callback was delivered out of tick order or from another world epoch.
    OutOfOrderPublication,
    /// The host rejected a result acknowledgement.
    Host(HostError),
}

/// Result of delivering one complete callback to the local phone view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalDelivery {
    /// Number of committed tick publications observed by the view.
    pub publications: usize,
    /// Highest acknowledged contiguous input sequence for this local peer.
    pub acknowledged_through: u64,
    /// Earlier publications may be delivered even when a later tick faulted.
    /// The caller must tear down a terminal host after using this report.
    pub terminal_fault: Option<HostError>,
}

/// One local peer's delivery cursor. Multiplayer transport has independent
/// receipt semantics and must not acknowledge server results here.
pub struct LocalReceiptCursor {
    peer: PeerId,
    acknowledged_through: u64,
    world_epoch: WorldEpoch,
    last_tick: u64,
    poisoned: bool,
}

impl LocalReceiptCursor {
    /// Start after the caller has handled and acknowledged any initial
    /// baseline outcomes. The host must have no unhandled earlier reports.
    pub fn new<G: GameRules>(
        peer: PeerId,
        host: &HeadlessHost<G>,
        initial_acknowledged_through: u64,
    ) -> Self {
        Self {
            peer,
            acknowledged_through: initial_acknowledged_through,
            world_epoch: host.authority().snapshot().epoch(),
            last_tick: host.authority().tick_index(),
            poisoned: false,
        }
    }

    /// Deliver every publication in tick order, then acknowledge each handled
    /// contiguous prefix. `observe` must be infallible and handle the complete
    /// publication, including game/timing rejections and events. This method
    /// rejects a second peer or skipped/reordered tick before calling it.
    /// Terminal reports deliver prior commits once but skip acknowledgement
    /// because the host is faulted. The callback stays with the caller so its
    /// clock overload and backpressure report remains observable.
    ///
    /// # Errors
    ///
    /// Returns a receipt error if the cursor is poisoned, a publication is out
    /// of order or belongs to another peer, or the host rejects acknowledgement.
    pub fn deliver<G: GameRules>(
        &mut self,
        host: &mut HeadlessHost<G>,
        callback: &GameHostCallback<G>,
        mut observe: impl FnMut(&HostPublication<G::Entity, G::Event, G::Rejection>),
    ) -> Result<LocalDelivery, LocalReceiptError> {
        if self.poisoned {
            return Err(LocalReceiptError::Poisoned);
        }
        let terminal_fault = callback.terminal_fault;
        let mut publications = 0;
        for publication in &callback.publications {
            let (snapshot, results, events, watermarks) = match publication {
                HostPublication::Local(tick) => (
                    &tick.snapshot,
                    tick.group.results.as_slice(),
                    tick.group.events.as_slice(),
                    tick.group.resolved_through.as_slice(),
                ),
                HostPublication::Replicated {
                    snapshot, admitted, ..
                } => (
                    snapshot,
                    admitted.results.as_slice(),
                    admitted.events.as_slice(),
                    admitted.resolved_through.as_slice(),
                ),
                HostPublication::ReplicatedCharged {
                    snapshot, admitted, ..
                } => (
                    snapshot,
                    admitted.results(),
                    admitted.events(),
                    admitted.resolved_through(),
                ),
            };
            if snapshot.epoch() != self.world_epoch
                || self.last_tick.checked_add(1) != Some(snapshot.tick())
            {
                return Err(LocalReceiptError::OutOfOrderPublication);
            }
            if results.iter().any(|result| result.origin.peer != self.peer)
                || events.iter().any(|event| event.origin.peer != self.peer)
                || watermarks.iter().any(|(peer, _)| *peer != self.peer)
            {
                return Err(LocalReceiptError::UnexpectedPeer);
            }
            // If the observer unwinds, or acknowledgement fails after it has
            // run, this cursor cannot safely replay a side effect.
            self.poisoned = true;
            observe(publication);
            if terminal_fault.is_none()
                && let Some((_, through)) = watermarks.first()
                && *through > self.acknowledged_through
            {
                host.acknowledge_results(self.peer, *through)
                    .map_err(LocalReceiptError::Host)?;
                self.acknowledged_through = *through;
            }
            self.last_tick = snapshot.tick();
            self.poisoned = false;
            publications += 1;
        }
        Ok(LocalDelivery {
            publications,
            acknowledged_through: self.acknowledged_through,
            terminal_fault,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use thrum_core::{
        ActionRejection, ActorId, Authority, AuthorityId, ClockConfig, HostBudget, Input,
        InputSchedule, Origin, PreparedCommit, PreparedEdits, ResultDisposition, RuleContext,
        SessionId, World, WorldEpoch,
    };

    struct RejectingRule;

    impl GameRules for RejectingRule {
        type Entity = ();
        type Action = u8;
        type Change = ();
        type Rejection = u8;
        type Event = ();

        fn decide(
            &self,
            _world: &World<()>,
            _actor: ActorId,
            action: &u8,
            _context: &mut RuleContext<'_>,
        ) -> Result<(), u8> {
            assert!(*action != 1, "test rule fault");
            Err(1)
        }

        fn prepare(
            &self,
            _world: &World<()>,
            _change: (),
            _context: &mut RuleContext<'_>,
        ) -> PreparedCommit<(), ()> {
            PreparedCommit {
                edits: PreparedEdits::None,
                event: None,
            }
        }
    }

    const PEER: PeerId = PeerId(1);
    const ACTOR: ActorId = ActorId(2);

    fn input(sequence: u64, schedule: InputSchedule) -> Input<u8> {
        Input {
            origin: Origin {
                peer: PEER,
                connection_epoch: 1,
                sequence,
            },
            actor: ACTOR,
            schedule,
            action: 0,
        }
    }

    #[test]
    fn local_receipts_keep_rejections_live_past_256_and_during_paused_drain() {
        let origin = Instant::now();
        let mut authority = Authority::new(
            AuthorityId::Standalone(SessionId(1)),
            RejectingRule,
            World::new(WorldEpoch(1)),
        );
        authority.register_peer(PEER, 1, ACTOR).unwrap();
        let (mut host, baseline) = HeadlessHost::with_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(4),
            },
            authority,
        )
        .unwrap();
        assert_eq!(baseline.results, Vec::new());
        let mut receipts = LocalReceiptCursor::new(PEER, &host, 0);
        let mut seen = 0;
        let mut timing_rejections = 0;
        let mut now = origin;
        for sequence in 1..=257 {
            let observed_tick = if sequence % 50 == 0 {
                0
            } else {
                host.authority().tick_index()
            };
            host.submit(input(sequence, InputSchedule::NextTick { observed_tick }))
                .unwrap();
            now += Duration::from_nanos(16_666_667);
            let report = host.on_callback(now).unwrap();
            receipts
                .deliver(&mut host, &report, |publication| {
                    let HostPublication::Replicated { admitted, .. } = publication else {
                        panic!("expected in-memory replica publication");
                    };
                    for result in &admitted.results {
                        assert!(matches!(result.disposition, ResultDisposition::Rejected(_)));
                        if matches!(
                            result.disposition,
                            ResultDisposition::Rejected(ActionRejection::StaleObservation)
                        ) {
                            timing_rejections += 1;
                        }
                        seen += 1;
                    }
                })
                .unwrap();
        }
        assert_eq!(seen, 257);
        assert_eq!(timing_rejections, 5);
        assert_eq!(receipts.acknowledged_through, 257);

        let due = host.authority().tick_index() + 3;
        host.submit(input(258, InputSchedule::Timed { intended_tick: due }))
            .unwrap();
        now += Duration::from_millis(100);
        let first = host.on_callback(now).unwrap();
        assert_eq!(first.clock.completed, 1);
        assert!(first.clock.queued > 0);
        receipts.deliver(&mut host, &first, |_| {}).unwrap();
        host.pause();
        now += Duration::from_secs(600);
        while receipts.acknowledged_through < 258 {
            let drained = host.on_callback(now).unwrap();
            assert_eq!(drained.clock.admitted, 0);
            receipts
                .deliver(&mut host, &drained, |publication| {
                    let HostPublication::Replicated { admitted, .. } = publication else {
                        panic!("expected in-memory replica publication");
                    };
                    seen += admitted.results.len();
                })
                .unwrap();
        }
        assert_eq!(seen, 258);
        assert_eq!(receipts.acknowledged_through, 258);
        host.submit(input(
            259,
            InputSchedule::NextTick {
                observed_tick: host.authority().tick_index(),
            },
        ))
        .expect_err("paused host must refuse new input");
    }

    #[test]
    fn held_reports_must_be_delivered_in_order_and_only_once() {
        let origin = Instant::now();
        let mut authority = Authority::new(
            AuthorityId::Standalone(SessionId(2)),
            RejectingRule,
            World::new(WorldEpoch(2)),
        );
        authority.register_peer(PEER, 1, ACTOR).unwrap();
        let (mut host, baseline) = HeadlessHost::with_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(4),
            },
            authority,
        )
        .unwrap();
        assert_eq!(baseline.results, Vec::new());
        let mut receipts = LocalReceiptCursor::new(PEER, &host, 0);
        host.submit(input(1, InputSchedule::NextTick { observed_tick: 0 }))
            .unwrap();
        let first = host
            .on_callback(origin + Duration::from_millis(20))
            .unwrap();
        host.submit(input(2, InputSchedule::NextTick { observed_tick: 1 }))
            .unwrap();
        let second = host
            .on_callback(origin + Duration::from_millis(40))
            .unwrap();

        let mut observed = 0;
        assert_eq!(
            receipts.deliver(&mut host, &second, |_| observed += 1),
            Err(LocalReceiptError::OutOfOrderPublication)
        );
        assert_eq!(observed, 0);
        receipts
            .deliver(&mut host, &first, |_| observed += 1)
            .unwrap();
        receipts
            .deliver(&mut host, &second, |_| observed += 1)
            .unwrap();
        assert_eq!(observed, 2);
        assert_eq!(receipts.acknowledged_through, 2);
        assert_eq!(
            receipts.deliver(&mut host, &first, |_| observed += 1),
            Err(LocalReceiptError::OutOfOrderPublication)
        );
        assert_eq!(observed, 2);
    }

    #[test]
    fn late_acknowledgement_error_poison_prevents_duplicate_observation() {
        let origin = Instant::now();
        let mut authority = Authority::new(
            AuthorityId::Standalone(SessionId(3)),
            RejectingRule,
            World::new(WorldEpoch(3)),
        );
        authority.register_peer(PEER, 1, ACTOR).unwrap();
        let (mut host, baseline) = HeadlessHost::with_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(4),
            },
            authority,
        )
        .unwrap();
        assert_eq!(baseline.results, Vec::new());
        let mut receipts = LocalReceiptCursor::new(PEER, &host, 0);
        host.submit(input(1, InputSchedule::NextTick { observed_tick: 0 }))
            .unwrap();
        let first = host
            .on_callback(origin + Duration::from_millis(20))
            .unwrap();
        let mut faulting = input(2, InputSchedule::NextTick { observed_tick: 1 });
        faulting.action = 1;
        host.submit(faulting).unwrap();
        let fault = host.on_callback(origin + Duration::from_millis(40));
        assert!(fault.is_err() || fault.unwrap().terminal_fault.is_some());

        let mut observed = 0;
        assert_eq!(
            receipts.deliver(&mut host, &first, |_| observed += 1),
            Err(LocalReceiptError::Host(HostError::Faulted))
        );
        assert_eq!(observed, 1);
        assert_eq!(
            receipts.deliver(&mut host, &first, |_| observed += 1),
            Err(LocalReceiptError::Poisoned)
        );
        assert_eq!(observed, 1);
    }

    #[test]
    fn terminal_report_delivers_earlier_result_once_without_acknowledgement() {
        let origin = Instant::now();
        let mut authority = Authority::new(
            AuthorityId::Standalone(SessionId(4)),
            RejectingRule,
            World::new(WorldEpoch(4)),
        );
        authority.register_peer(PEER, 1, ACTOR).unwrap();
        let (mut host, baseline) = HeadlessHost::with_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
        )
        .unwrap();
        assert_eq!(baseline.results, Vec::new());
        let mut receipts = LocalReceiptCursor::new(PEER, &host, 0);
        host.submit(input(1, InputSchedule::NextTick { observed_tick: 0 }))
            .unwrap();
        let mut faulting = input(2, InputSchedule::Timed { intended_tick: 2 });
        faulting.action = 1;
        host.submit(faulting).unwrap();
        let report = host
            .on_callback(origin + Duration::from_millis(40))
            .unwrap();
        assert!(report.terminal_fault.is_some());
        assert_eq!(report.publications.len(), 1);

        let mut observed = 0;
        let delivery = receipts
            .deliver(&mut host, &report, |_| observed += 1)
            .unwrap();
        assert_eq!(observed, 1);
        assert_eq!(delivery.acknowledged_through, 0);
        assert_eq!(delivery.publications, 1);
        assert!(delivery.terminal_fault.is_some());
        assert_eq!(
            receipts.deliver(&mut host, &report, |_| observed += 1),
            Err(LocalReceiptError::OutOfOrderPublication)
        );
        assert_eq!(observed, 1);
    }
}
