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
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use thrum_cook::{LinkedRuleError, SceneTarget, cook_entry_scene, verify_linked_rule_source};
    use thrum_core::{Admission, ResultDisposition};
    use thrum_source::{
        NativeSourceStore, SourcePath, SourceSpacePlan, registry_stage::validate_staged_candidate,
        stage_declared_tree_with_plan,
    };

    const LINKED_GAME: &[u8] = include_bytes!("game.rs");
    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const HISTORY_ID: &str = "6a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct SourceFixture(PathBuf);

    impl SourceFixture {
        fn new() -> Self {
            let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "thrum-i1-linked-{}-{}",
                std::process::id(),
                sequence
            ));
            fs::create_dir(&root).unwrap();
            for path in [".engine", "game", "scenes", "schemas"] {
                fs::create_dir(root.join(path)).unwrap();
            }
            fs::write(
                root.join("project.json"),
                format!(
                    "{{\"id\":\"{PROJECT_ID}\",\"type\":\"core.project\",\"schema\":2,\"fields\":{{\"project.format\":1,\"project.records\":[\"scenes/main.scene.json\"],\"project.schemas\":[],\"project.registryHistory\":\"schemas/history.registry.json\",\"project.entryScenes\":[\"{SCENE_ID}\"]}}}}"
                ),
            )
            .unwrap();
            fs::write(
                root.join("scenes/main.scene.json"),
                format!(
                    "{{\"id\":\"{SCENE_ID}\",\"type\":\"core.scene\",\"schema\":1,\"fields\":{{\"scene.label\":\"Linked tap game\"}}}}"
                ),
            )
            .unwrap();
            fs::write(
                root.join("schemas/history.registry.json"),
                format!(
                    "{{\"id\":\"{HISTORY_ID}\",\"type\":\"core.registry-history\",\"schema\":1,\"fields\":{{\"registry.fields\":[],\"registry.versions\":[]}}}}"
                ),
            )
            .unwrap();
            fs::write(root.join("game/rules.rs"), LINKED_GAME).unwrap();
            Self(root)
        }

        fn paths() -> Vec<SourcePath> {
            [
                "game/rules.rs",
                "project.json",
                "scenes/main.scene.json",
                "schemas/history.registry.json",
            ]
            .map(|path| SourcePath::parse(path).unwrap())
            .to_vec()
        }

        fn plan() -> SourceSpacePlan {
            SourceSpacePlan {
                main_target_bytes: 1024 * 1024 * 1024,
                wal_stop_bytes: 64 * 1024 * 1024,
                temporary_target_bytes: 64 * 1024 * 1024,
                free_headroom_bytes: 0,
            }
        }

        fn admit(&self, store: &mut NativeSourceStore, generation: i64, key: i64) -> i64 {
            let stage =
                stage_declared_tree_with_plan(&self.0, &Self::paths(), Self::plan()).unwrap();
            let candidate = validate_staged_candidate(&stage, &[]).unwrap();
            store
                .admit(generation, 1, key, [key as u8; 32], &candidate)
                .unwrap()
                .generation
        }
    }

    impl Drop for SourceFixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

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

    #[test]
    fn admitted_scene_and_linked_rules_reach_same_headless_authority() {
        let fixture = SourceFixture::new();
        let mut store = NativeSourceStore::open(
            &fixture.0.join(".engine/source.sqlite"),
            SourceFixture::plan(),
        )
        .unwrap();
        let generation = fixture.admit(&mut store, 0, 1);
        let rules_path = SourcePath::parse("game/rules.rs").unwrap();
        let linked_hash: [u8; 32] = Sha256::digest(LINKED_GAME).into();
        let mut pin = store.pin_current(generation).unwrap();
        let product = cook_entry_scene(&mut pin, SceneTarget::Headless).unwrap();
        assert_eq!(product.label(), Some("Linked tap game"));
        verify_linked_rule_source(&mut pin, &rules_path, LINKED_GAME.len() as u64, linked_hash)
            .unwrap();
        pin.close().unwrap();

        let origin = Instant::now();
        let (mut host, _) = create_host(origin);
        fs::write(fixture.0.join("game/rules.rs"), b"new unlinked rules").unwrap();
        let next_generation = fixture.admit(&mut store, generation, 2);
        let mut next_pin = store.pin_current(next_generation).unwrap();
        assert!(matches!(
            verify_linked_rule_source(
                &mut next_pin,
                &rules_path,
                LINKED_GAME.len() as u64,
                linked_hash
            ),
            Err(LinkedRuleError::Mismatch)
        ));
        next_pin.close().unwrap();

        assert_eq!(host.submit(tap_input(1, 0)), Ok(Admission::Queued));
        host.on_callback(origin + Duration::from_millis(20))
            .unwrap();
        assert_eq!(host.replica().unwrap().get(COUNTER).unwrap().taps, 1);
    }
}
