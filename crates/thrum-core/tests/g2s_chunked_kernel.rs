//! G2S parallel fitness of the chunked in-place kernel through the public
//! runtime: forced Rayon runs independent chunks at the same time, and every
//! mode commits, refuses, restores and rewinds the same complete state. A
//! runtime whose commit unwound refuses reuse instead of re-applying a tick.

use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use thrum_core::{
    Handle, StableId, WorldEpoch,
    sim::{
        Access, AccessKey, AccessMode, ComponentRegistry, ComponentSchemaId, EditWriter,
        ExecutionMode, Executor, GameRuntime, PublicationMode, ReadPermit, RegisteredSystem, Stage,
        SystemId, SystemSpec, TickError, TickGame, WorldRead, WorldTxn, WritePermit, WriteRead2,
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Height(f64);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Drift(f64);

const CAPACITY: usize = 8_192;
const MOVE: SystemId = SystemId(1);
const NUDGE: SystemId = SystemId(2);
const EXCLUDED: u32 = 4;

struct Page;

struct Candidate {
    motion: WriteRead2<Height, Drift>,
    changed: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Refused;

/// Measures how many chunk callbacks overlap. Leading rows of each chunk wait
/// briefly for a second chunk to arrive, which only parallel dispatch can
/// satisfy; serial dispatch times out with a peak of one.
#[derive(Default)]
struct Probe {
    enabled: AtomicBool,
    inside: AtomicUsize,
    peak: AtomicUsize,
    waits: AtomicUsize,
}

impl Probe {
    fn enter(&self) {
        let now = self.inside.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        // A few bounded waits suffice: parallel chunks arrive within one.
        let waiting = self.waits.fetch_add(1, Ordering::SeqCst) < 4;
        let deadline = Instant::now() + Duration::from_millis(250);
        while waiting && self.peak.load(Ordering::SeqCst) < 2 && Instant::now() < deadline {
            std::hint::spin_loop();
        }
        self.inside.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct Rules {
    refuse_slot: Option<u32>,
    panic_apply: bool,
    /// Rows the kernel leaves unchanged (`Ok(false)`): every fifth slot.
    skip_fifths: bool,
    /// An earlier Input system writes these rows before the kernel visits.
    nudge: Option<(WritePermit<Height>, Vec<Handle>)>,
    probe: Probe,
}

impl Rules {
    #[expect(
        clippy::unnecessary_wraps,
        reason = "registered direct systems share one fallible callback signature"
    )]
    fn nudge(
        &self,
        world: &mut WorldTxn<'_>,
        _executor: Executor,
        _tick: u64,
        _candidate: &mut Candidate,
        _edits: &mut EditWriter<'_, Candidate>,
    ) -> Result<(), Refused> {
        if let Some((permit, rows)) = &self.nudge {
            for (offset, handle) in rows.iter().enumerate() {
                world
                    .put(permit, *handle, Height(1_000.0 + offset as f64))
                    .expect("nudge row");
            }
        }
        Ok(())
    }

    fn simulate(
        &self,
        world: &mut WorldTxn<'_>,
        executor: Executor,
        _tick: u64,
        candidate: &mut Candidate,
        _edits: &mut EditWriter<'_, Candidate>,
    ) -> Result<(), Refused> {
        let motion = candidate.motion;
        let probe = self.probe.enabled.load(Ordering::SeqCst);
        let skip_fifths = self.skip_fifths;
        candidate.changed = world.visit_chunked_indexed(
            &motion,
            executor,
            &self.refuse_slot,
            |_, mark| mark(EXCLUDED as usize),
            |_, _, _, _, _| true,
            |refuse, handle, _, height, drift| {
                if probe && handle.slot % 1_024 < 8 {
                    self.probe.enter();
                }
                if skip_fifths && handle.slot.is_multiple_of(5) {
                    return Ok(false);
                }
                height.0 += drift.0;
                if *refuse == Some(handle.slot) {
                    return Err(Refused);
                }
                Ok(true)
            },
        )?;
        Ok(())
    }
}

impl TickGame<Page> for Rules {
    type Error = Refused;
    type Candidate = Candidate;
    type ParallelOutput = ();

    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.changed = 0;
    }

    fn reset_parallel_output(&self, _output: &mut ()) {}

    fn merge_parallel(
        &self,
        _system: SystemId,
        _output: &mut (),
        _candidate: &mut Candidate,
        _tick: u64,
        _edits: &mut EditWriter<'_, Candidate>,
    ) {
    }

    fn apply(&mut self, _world: &WorldRead<'_>, _tick: u64, _candidate: &Candidate) {
        assert!(!self.panic_apply, "game apply panicked after commit");
    }
}

struct Fixture {
    runtime: GameRuntime<Page, Rules>,
    rules: Rules,
    candidate: Candidate,
    handles: Vec<Handle>,
    height: ReadPermit<Height>,
}

fn is_mover(index: u32) -> bool {
    !index.is_multiple_of(3)
}

fn fixture(mode: ExecutionMode) -> Fixture {
    let mut registry = ComponentRegistry::default();
    let height = registry
        .register::<Height>(ComponentSchemaId(0xC1))
        .expect("height schema");
    let drift = registry
        .register_indexed::<Drift>(ComponentSchemaId(0xC2), |drift| drift.0 != 0.0)
        .expect("drift schema");
    registry
        .select_activity_partition::<Drift>()
        .expect("dense movers");
    let systems = vec![
        RegisteredSystem::direct(
            SystemSpec {
                id: NUDGE,
                stage: Stage::Input,
                accesses: vec![Access {
                    key: AccessKey::Component(height),
                    mode: AccessMode::Write,
                }],
                after: vec![],
            },
            Rules::nudge,
        ),
        RegisteredSystem::direct(
            SystemSpec {
                id: MOVE,
                stage: Stage::Simulate,
                accesses: vec![
                    Access {
                        key: AccessKey::Component(height),
                        mode: AccessMode::Write,
                    },
                    Access {
                        key: AccessKey::Component(drift),
                        mode: AccessMode::Read,
                    },
                ],
                after: vec![],
            },
            Rules::simulate,
        ),
    ];
    let mut runtime =
        GameRuntime::new(WorldEpoch(31), CAPACITY, registry, systems, || Page).expect("setup");
    runtime.set_execution_mode(mode);
    // Movers interleave with statics so the activity partition reorders rows.
    let handles = (0..u32::try_from(CAPACITY).expect("capacity"))
        .map(|index| {
            runtime
                .seed(StableId(u128::from(index)), |entity| {
                    entity.add(Height(f64::from(index)))?;
                    let speed = if is_mover(index) {
                        0.25 + f64::from(index % 7)
                    } else {
                        0.0
                    };
                    entity.add(Drift(speed))?;
                    Ok(())
                })
                .expect("seed")
        })
        .collect();
    let mut candidate = Candidate {
        motion: runtime
            .write_read2_indexed::<Height, Drift>(MOVE)
            .expect("indexed motion"),
        changed: 0,
    };
    let height = runtime.read_permit(MOVE).expect("height read");
    let mut rules = Rules::default();
    runtime.publish_initial(&mut rules, &mut candidate);
    Fixture {
        runtime,
        rules,
        candidate,
        handles,
        height,
    }
}

impl Fixture {
    fn heights(&self) -> Vec<u64> {
        let world = self.runtime.read();
        self.handles
            .iter()
            .map(|handle| {
                world
                    .with(&self.height, *handle, |value| value.0.to_bits())
                    .expect("live height")
            })
            .collect()
    }

    fn step(&mut self) -> Result<u64, TickError<Refused>> {
        self.runtime.step(&mut self.rules, &mut self.candidate)
    }
}

#[test]
fn forced_rayon_runs_independent_chunks_at_the_same_time() {
    let mut forced = fixture(ExecutionMode::RayonForced);
    forced.rules.probe.enabled.store(true, Ordering::SeqCst);
    forced.step().expect("forced tick");
    assert!(
        forced.rules.probe.peak.load(Ordering::SeqCst) >= 2,
        "forced Rayon never overlapped two chunk callbacks"
    );
    let mut serial = fixture(ExecutionMode::Serial);
    serial.rules.probe.enabled.store(true, Ordering::SeqCst);
    serial.step().expect("serial tick");
    assert_eq!(serial.rules.probe.peak.load(Ordering::SeqCst), 1);
    assert_eq!(forced.heights(), serial.heights());
}

fn run(mode: ExecutionMode) -> (Vec<u64>, usize) {
    let mut world = fixture(mode);
    let initial = world.heights();
    for _ in 0..3 {
        world.step().expect("moving tick");
    }
    let moved = world.candidate.changed;
    let committed = world.heights();
    for (index, (now, before)) in committed.iter().zip(&initial).enumerate() {
        let index = u32::try_from(index).expect("small");
        let visited = is_mover(index) && index != EXCLUDED;
        assert_eq!(now != before, visited, "row {index} visit set");
    }

    // Refuse from the last mover: every other chunk has written tentatively,
    // and the abort restores all of them.
    let last = (0..u32::try_from(CAPACITY).expect("capacity"))
        .rev()
        .find(|index| is_mover(*index))
        .expect("last mover");
    world.rules.refuse_slot = Some(world.handles[last as usize].slot);
    assert_eq!(world.step(), Err(TickError::Game(Refused)));
    assert_eq!(world.runtime.tick(), 3);
    assert_eq!(world.heights(), committed, "refusal restores every chunk");

    // A recorded tick merges every chunk's preimages, so rewind restores them.
    world.rules.refuse_slot = None;
    let mut undo = [world.runtime.new_undo()];
    world
        .runtime
        .step_recording(
            &mut world.rules,
            &mut world.candidate,
            &mut undo[0],
            PublicationMode::Defer,
        )
        .expect("recorded tick");
    let recorded = world.heights();
    assert_ne!(recorded, committed);
    world
        .runtime
        .rewind_many(&mut undo)
        .expect("rewind one tick");
    assert_eq!(world.heights(), committed, "rewind restores every chunk");
    world.step().expect("replayed tick");
    assert_eq!(
        world.heights(),
        recorded,
        "replay matches the recorded tick"
    );
    (recorded, moved)
}

#[test]
fn serial_rayon_and_forced_chunks_commit_refuse_and_rewind_identically() {
    let serial = run(ExecutionMode::Serial);
    let movers = (0..CAPACITY)
        .filter(|index| !index.is_multiple_of(3))
        .count();
    assert_eq!(serial.1, movers - 1, "every mover except the excluded one");
    assert_eq!(run(ExecutionMode::Rayon), serial);
    assert_eq!(run(ExecutionMode::RayonForced), serial);
}

#[test]
fn a_runtime_whose_commit_unwound_refuses_reuse() {
    let mut world = fixture(ExecutionMode::Serial);
    world.step().expect("first tick");
    let capture = world.runtime.capture();
    let committed = world.heights();
    world.rules.panic_apply = true;
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| world.step()));
    assert!(unwound.is_err(), "apply panic propagates");
    // The world already holds the next tick's values; stepping again would
    // apply that tick's rules twice under the old tick number.
    world.rules.panic_apply = false;
    let reused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| world.step()));
    assert!(reused.is_err(), "a poisoned runtime must refuse to step");
    let mut undo = [world.runtime.new_undo()];
    let rewound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.runtime.rewind_many(&mut undo)
    }));
    assert!(rewound.is_err(), "a poisoned runtime must refuse to rewind");
    // A full restore replaces the half-committed state and recovers the runtime.
    world.runtime.restore(&capture, || Page).expect("restore");
    world
        .runtime
        .publish_initial(&mut world.rules, &mut world.candidate);
    assert_eq!(world.heights(), committed);
    assert_eq!(world.step(), Ok(2));
}

/// Rows an earlier system wrote keep that system's preimage, and rows the
/// kernel leaves unchanged keep none: refusal and rewind both restore the
/// exact pre-tick world in every mode.
fn earlier_writer_and_unchanged_rows(mode: ExecutionMode) -> (Vec<u64>, usize) {
    let mut world = fixture(mode);
    let before = world.heights();
    // Movers in three different chunks.
    let rows: Vec<Handle> = [1_u32, 4_097, 8_191]
        .iter()
        .map(|index| world.handles[*index as usize])
        .collect();
    assert!(rows.iter().all(|handle| is_mover(handle.slot)));
    let permit = world.runtime.write_permit(NUDGE).expect("nudge write");
    world.rules.nudge = Some((permit, rows.clone()));
    world.rules.skip_fifths = true;

    let last = (0..u32::try_from(CAPACITY).expect("capacity"))
        .rev()
        .find(|index| is_mover(*index) && !index.is_multiple_of(5))
        .expect("last changed mover");
    world.rules.refuse_slot = Some(world.handles[last as usize].slot);
    assert_eq!(world.step(), Err(TickError::Game(Refused)));
    assert_eq!(world.heights(), before, "refusal restores nudged rows too");

    world.rules.refuse_slot = None;
    let mut undo = [world.runtime.new_undo()];
    world
        .runtime
        .step_recording(
            &mut world.rules,
            &mut world.candidate,
            &mut undo[0],
            PublicationMode::Defer,
        )
        .expect("recorded tick");
    let recorded = world.heights();
    let changed = world.candidate.changed;
    for (index, (now, was)) in recorded.iter().zip(&before).enumerate() {
        let index = u32::try_from(index).expect("small");
        let nudged = rows.iter().position(|handle| handle.slot == index);
        let moved = is_mover(index) && index != EXCLUDED && !index.is_multiple_of(5);
        let drift = if moved {
            0.25 + f64::from(index % 7)
        } else {
            0.0
        };
        let expected = match nudged {
            Some(offset) => (1_000.0 + offset as f64 + drift).to_bits(),
            None => (f64::from_bits(*was) + drift).to_bits(),
        };
        assert_eq!(*now, expected, "row {index}");
    }
    world.runtime.rewind_many(&mut undo).expect("rewind");
    assert_eq!(
        world.heights(),
        before,
        "rewind restores the pre-nudge values"
    );
    (recorded, changed)
}

#[test]
fn earlier_writers_and_unchanged_rows_restore_exactly_in_every_mode() {
    let serial = earlier_writer_and_unchanged_rows(ExecutionMode::Serial);
    let expected = (0..u32::try_from(CAPACITY).expect("capacity"))
        .filter(|index| is_mover(*index) && *index != EXCLUDED && !index.is_multiple_of(5))
        .count();
    assert_eq!(serial.1, expected, "unchanged rows are not counted");
    assert_eq!(
        earlier_writer_and_unchanged_rows(ExecutionMode::Rayon),
        serial
    );
    assert_eq!(
        earlier_writer_and_unchanged_rows(ExecutionMode::RayonForced),
        serial
    );
}
