//! One bundled local game: a controlled touch increments one authoritative
//! object and publishes one event. The same rules can run under match authority.

use std::time::{Duration, Instant};

use thrum_core::{
    ActorId, AdmittedGroup, Authority, AuthorityId, ClockConfig, GameRules, HeadlessHost,
    HostBudget, Input, InputSchedule, Origin, PeerId, PreparedCommit, PreparedEdits,
    ResultDisposition, RuleContext, SessionId, StableId, World, WorldEdit, WorldEpoch,
};
use thrum_mobile_shell::LocalReceiptCursor;

pub const ACTOR: ActorId = ActorId(1);
pub const PEER: PeerId = PeerId(1);
pub const COUNTER: StableId = StableId(1);
#[cfg(any(target_os = "ios", target_os = "android"))]
pub const BUNDLED_GAME_ID: &str = "i1-tap-counter-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Counter {
    pub taps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Tap,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rejection {
    WrongActor,
    Overflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    Tapped,
}

pub struct Rules;

/// Finite local outcome view updated before the receipt cursor acknowledges.
#[derive(Default)]
pub struct LocalView {
    pub handled_results: u64,
    pub rejected_results: u64,
    pub last_event_id: Option<u64>,
}

impl LocalView {
    pub fn observe(&mut self, admitted: &AdmittedGroup<Event, Rejection>) {
        for result in &admitted.results {
            self.handled_results = self.handled_results.saturating_add(1);
            if matches!(result.disposition, ResultDisposition::Rejected(_)) {
                self.rejected_results = self.rejected_results.saturating_add(1);
            }
        }
        for event in &admitted.events {
            self.last_event_id = Some(event.id);
        }
    }
}

impl GameRules for Rules {
    type Entity = Counter;
    type Action = Action;
    type Change = u64;
    type Rejection = Rejection;
    type Event = Event;

    fn decide(
        &self,
        world: &World<Counter>,
        actor: ActorId,
        _action: &Action,
        _context: &mut RuleContext<'_>,
    ) -> Result<u64, Rejection> {
        if actor != ACTOR {
            return Err(Rejection::WrongActor);
        }
        let handle = world.handle_for(COUNTER).ok_or(Rejection::WrongActor)?;
        let old = world.get(handle).ok_or(Rejection::WrongActor)?;
        old.state.taps.checked_add(1).ok_or(Rejection::Overflow)
    }

    fn prepare(
        &self,
        world: &World<Counter>,
        next: u64,
        _context: &mut RuleContext<'_>,
    ) -> PreparedCommit<Counter, Event> {
        let handle = world.handle_for(COUNTER).expect("preloaded counter exists");
        PreparedCommit {
            edits: PreparedEdits::One(WorldEdit::Replace(handle, Counter { taps: next })),
            event: Some(Event::Tapped),
        }
    }
}

pub fn create_host(origin: Instant) -> (HeadlessHost<Rules>, LocalReceiptCursor) {
    let mut world = World::new(WorldEpoch(1));
    world
        .spawn(COUNTER, Counter { taps: 0 })
        .expect("one object");
    let mut authority = Authority::new(AuthorityId::Standalone(SessionId(1)), Rules, world);
    authority.register_peer(PEER, 1, ACTOR).expect("one peer");
    let (host, baseline) = HeadlessHost::with_replica(
        origin,
        ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(5),
            other_work: Duration::from_millis(1),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
    )
    .expect("preloaded host");
    assert!(baseline.results.is_empty() && baseline.events.is_empty());
    let receipts = LocalReceiptCursor::new(PEER, &host, 0);
    (host, receipts)
}

pub fn tap_input(sequence: u64, observed_tick: u64) -> Input<Action> {
    Input {
        origin: Origin {
            peer: PEER,
            connection_epoch: 1,
            sequence,
        },
        actor: ACTOR,
        schedule: InputSchedule::NextTick { observed_tick },
        action: Action::Tap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thrum_core::{Admission, ResultDisposition};

    #[test]
    fn bundled_game_uses_one_authority_and_replica_result() {
        let origin = Instant::now();
        let (mut host, mut receipts) = create_host(origin);
        assert_eq!(host.submit(tap_input(1, 0)), Ok(Admission::Queued));
        let callback = host
            .on_callback(origin + Duration::from_millis(20))
            .unwrap();
        let mut saw_result = false;
        let mut view = LocalView::default();
        receipts
            .deliver(&mut host, &callback, |publication| {
                if let thrum_core::HostPublication::Replicated { admitted, .. } = publication {
                    view.observe(admitted);
                    saw_result = admitted
                        .results
                        .iter()
                        .any(|result| result.disposition == ResultDisposition::Committed);
                }
            })
            .unwrap();
        assert!(saw_result);
        assert_eq!(view.handled_results, 1);
        assert_eq!(view.rejected_results, 0);
        assert_eq!(view.last_event_id, Some(1));
        assert_eq!(host.replica().unwrap().get(COUNTER).unwrap().taps, 1);
    }
}
