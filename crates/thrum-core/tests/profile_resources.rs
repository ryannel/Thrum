//! Profile resource preflight covers queued edits, tentative writes and merges.
use thrum_core::{
    Handle, StableId, WorldEpoch,
    profile::{Digest, Number, StateDecode, StateDecodeError, StateEncode, StateReader},
    sim::{
        Access, AccessKey, AccessMode, ComponentRegistry, ComponentSchemaId, Edit, EditError,
        EditWriter, ExecutionMode, Executor, GameRuntime, RegisteredSystem, RegisteredWorldError,
        Stage, SystemId, SystemSpec, TickError, TickGame, WorldRead, WorldTxn, WritePermit,
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Scalar(Number);
impl StateEncode for Scalar {
    fn encode(&self, digest: &mut Digest<'_>) {
        self.0.encode(digest);
    }
}
impl StateDecode for Scalar {
    fn decode(reader: &mut StateReader<'_>) -> Result<Self, StateDecodeError> {
        Ok(Self(Number::decode(reader)?))
    }
}

// Deliberately use the default recursive finiteness check over nested arrays.
#[derive(Clone, Debug, PartialEq)]
struct Resources([[Number; 2]; 2]);
impl StateEncode for Resources {
    fn encode(&self, digest: &mut Digest<'_>) {
        self.0.encode(digest);
    }
}
impl StateDecode for Resources {
    fn decode(reader: &mut StateReader<'_>) -> Result<Self, StateDecodeError> {
        Ok(Self(<[[Number; 2]; 2]>::decode(reader)?))
    }
}

struct Candidate {
    component: Scalar,
    resources: Resources,
    scratch: Number,
}

struct Rules {
    write: WritePermit<Scalar>,
    handle: Handle,
    next: Number,
    resources: Resources,
    applies: u64,
    profile: bool,
}

impl TickGame for Rules {
    type Error = RegisteredWorldError;
    type Candidate = Candidate;
    type ParallelOutput = Option<Number>;

    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.component = Scalar(Number::from_i32(
            7 + i32::try_from(self.applies).expect("small test count"),
        ));
        candidate.resources.clone_from(&self.resources);
        // Invalid derived scratch is deliberately outside authority validation.
        candidate.scratch = Number::from_i32(0) / Number::ZERO;
    }
    fn reset_parallel_output(&self, output: &mut Self::ParallelOutput) {
        *output = None;
    }
    fn merge_parallel(
        &self,
        _: SystemId,
        output: &mut Self::ParallelOutput,
        candidate: &mut Candidate,
        _: u64,
        _: &mut EditWriter<'_, Candidate>,
    ) {
        candidate.resources.0[1][1] = output.expect("callback output");
    }
    fn profile_resources<'a>(&self, candidate: &'a Candidate) -> Option<&'a dyn StateDecode> {
        self.profile.then_some(&candidate.resources)
    }
    fn apply(&mut self, _: &WorldRead<'_>, _: u64, candidate: &Candidate) {
        self.resources.clone_from(&candidate.resources);
        self.applies += 1;
    }
}

#[expect(clippy::unnecessary_wraps, reason = "registered callback signature")]
fn queued(
    game: &Rules,
    _: &WorldRead<'_>,
    _: Executor,
    _: u64,
    _: &mut Candidate,
    edits: &mut EditWriter<'_, Candidate>,
) -> Result<(), RegisteredWorldError> {
    edits.push(Edit::replace(&game.write, game.handle, 0, |c, _| {
        &c.component
    }));
    Ok(())
}

fn direct(
    game: &Rules,
    world: &mut WorldTxn<'_>,
    _: Executor,
    _: u64,
    candidate: &mut Candidate,
    _: &mut EditWriter<'_, Candidate>,
) -> Result<(), RegisteredWorldError> {
    world.put(&game.write, game.handle, candidate.component)?;
    Ok(())
}

#[expect(clippy::unnecessary_wraps, reason = "registered callback signature")]
fn resource(
    game: &Rules,
    _: &WorldRead<'_>,
    _: Executor,
    _: u64,
    output: &mut Option<Number>,
) -> Result<(), RegisteredWorldError> {
    *output = Some(game.next);
    Ok(())
}

fn setup(in_place: bool) -> (GameRuntime<Rules>, Rules, Candidate) {
    let mut registry = ComponentRegistry::default();
    let key = registry
        .register_profile::<Scalar>(ComponentSchemaId(123))
        .expect("schema");
    registry
        .select_publication::<Scalar>(false)
        .expect("publication");
    let spec = SystemSpec {
        id: SystemId(1),
        stage: Stage::Simulate,
        accesses: vec![Access {
            key: AccessKey::Component(key),
            mode: AccessMode::Write,
        }],
        after: vec![],
    };
    let component = if in_place {
        RegisteredSystem::direct(spec, direct)
    } else {
        RegisteredSystem::new(spec, queued)
    };
    let mut runtime = GameRuntime::new_published(
        WorldEpoch(1),
        4,
        registry,
        vec![
            component,
            RegisteredSystem::parallel(
                SystemSpec {
                    id: SystemId(2),
                    stage: Stage::Simulate,
                    accesses: vec![],
                    after: vec![SystemId(1)],
                },
                resource,
            ),
            RegisteredSystem::parallel(
                SystemSpec {
                    id: SystemId(3),
                    stage: Stage::Simulate,
                    accesses: vec![],
                    after: vec![SystemId(1)],
                },
                resource,
            ),
        ],
    )
    .expect("runtime");
    let handle = runtime
        .seed(StableId(987), |writer| {
            writer.add(Scalar(Number::ONE))?;
            Ok(())
        })
        .expect("seed");
    let resources = Resources([[Number::ZERO; 2]; 2]);
    let mut game = Rules {
        write: runtime.write_permit(SystemId(1)).expect("permit"),
        handle,
        next: Number::ONE,
        resources: resources.clone(),
        applies: 0,
        profile: true,
    };
    let mut candidate = Candidate {
        component: Scalar(Number::ZERO),
        resources,
        scratch: Number::ZERO,
    };
    runtime.publish_initial(&mut game, &mut candidate);
    (runtime, game, candidate)
}

fn digest(runtime: &GameRuntime<Rules>, game: &Rules) -> u64 {
    runtime
        .world_digest(&game.resources, |_, _| {})
        .expect("portable world")
}

#[test]
fn nonfinite_resources_refuse_atomically_after_merges_and_recover() {
    for in_place in [false, true] {
        for mode in [ExecutionMode::Serial, ExecutionMode::RayonForced] {
            let (mut runtime, mut game, mut candidate) = setup(in_place);
            runtime.set_execution_mode(mode);
            // First commit establishes a nonzero endpoint.
            assert_eq!(runtime.step(&mut game, &mut candidate), Ok(1));
            let before = digest(&runtime, &game);
            let endpoint = runtime.endpoint();
            let resources = game.resources.clone();
            let page = runtime.latest().expect("published page");
            let page_id = page.id();
            let applies = game.applies;
            for invalid in [
                Number::ONE / Number::ZERO,
                -Number::ONE / Number::ZERO,
                Number::ZERO / Number::ZERO,
            ] {
                game.next = invalid;
                assert_eq!(
                    runtime.step(&mut game, &mut candidate),
                    Err(TickError::Edit(EditError::World(
                        RegisteredWorldError::NonFiniteResource
                    )))
                );
                assert_eq!(runtime.endpoint(), endpoint);
                assert_eq!(game.resources, resources);
                assert_eq!(game.applies, applies);
                assert_eq!(digest(&runtime, &game), before);
                assert_eq!(runtime.latest().expect("same publication").id(), page_id);
                assert_eq!(
                    *page.component::<Scalar>(game.handle).expect("component"),
                    Scalar(Number::from_i32(7))
                );
                assert_eq!(
                    runtime.capture().tick(),
                    1,
                    "a capture after refusal names the committed tick"
                );
            }
            game.next = Number::from_i32(3);
            assert_eq!(runtime.step(&mut game, &mut candidate), Ok(2));
            assert_eq!(game.resources.0[1][1], game.next);
            assert_eq!(game.applies, applies + 1);
            assert_ne!(digest(&runtime, &game), before);
            assert_eq!(runtime.latest().expect("new page").id().tick, 2);
        }
    }
}

#[test]
fn nonprofile_resources_remain_unrestricted() {
    let (mut runtime, mut game, mut candidate) = setup(false);
    game.profile = false;
    game.next = Number::ONE / Number::ZERO;
    assert_eq!(runtime.step(&mut game, &mut candidate), Ok(1));
    assert!(!game.resources.0[1][1].is_finite());
}
