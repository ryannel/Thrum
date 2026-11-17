use thrum_core::{
    Handle, StableId, WorldEpoch,
    profile::{Digest, Number, StateEncode},
    sim::{
        Access, AccessKey, AccessMode, ComponentRegistry, ComponentSchemaId, Edit, EditWriter,
        Executor, GameRuntime, RegisteredSystem, RegisteredWorldError, Stage, SystemId, SystemSpec,
        TickGame, WorldRead, WritePermit,
    },
};

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Scalar(Number);
impl StateEncode for Scalar {
    fn encode(&self, digest: &mut Digest) {
        self.0.encode(digest);
    }
}

pub(crate) struct Candidate { component: Scalar, resource: Number }

pub(crate) struct Rules {
    pub write: WritePermit<Scalar>,
    pub handle: Handle,
    pub next: Number,
    pub resource: Number,
}
impl TickGame for Rules {
    type Error = RegisteredWorldError;
    type Candidate = Candidate;
    type ParallelOutput = ();
    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.component = Scalar(Number::ONE);
        candidate.resource = self.next;
    }
    fn reset_parallel_output(&self, (): &mut ()) {}
    fn merge_parallel(
        &self,
        _: SystemId,
        (): &mut (),
        _: &mut Candidate,
        _: u64,
        _: &mut EditWriter<'_, Candidate>,
    ) {
    }
    fn apply(&mut self, _: &WorldRead<'_>, _: u64, candidate: &Candidate) {
        self.resource = candidate.resource;
    }
}
#[expect(
    clippy::unnecessary_wraps,
    reason = "registered callbacks have a fallible signature"
)]
fn rule(
    game: &Rules,
    _: &WorldRead<'_>,
    _: Executor,
    _: u64,
    _: &mut Candidate,
    edits: &mut EditWriter<'_, Candidate>,
) -> Result<(), RegisteredWorldError> {
    edits.push(Edit::replace(&game.write, game.handle, 0, |value, _| &value.component));
    Ok(())
}
pub(crate) fn setup(epoch: u128) -> (GameRuntime<Rules>, Rules, Candidate) {
    let mut registry = ComponentRegistry::default();
    let key = registry
        .register_profile::<Scalar>(ComponentSchemaId(123))
        .expect("schema");
    registry
        .select_publication::<Scalar>(false)
        .expect("published scalar");
    let mut runtime = GameRuntime::new_published(
        WorldEpoch(epoch),
        4,
        registry,
        vec![RegisteredSystem::new(
            SystemSpec {
                id: SystemId(1),
                stage: Stage::Simulate,
                accesses: vec![Access {
                    key: AccessKey::Component(key),
                    mode: AccessMode::Write,
                }],
                after: vec![],
            },
            rule,
        )],
    )
    .expect("runtime");
    let handle = runtime
        .seed(StableId(987), |writer| {
            writer.add(Scalar(Number::ONE))?;
            Ok(())
        })
        .expect("seed");
    let mut game = Rules {
        write: runtime.write_permit(SystemId(1)).expect("permit"),
        handle,
        next: Number::ONE,
        resource: Number::ZERO,
    };
    let mut candidate = Candidate { component: Scalar(Number::ZERO), resource: Number::ZERO };
    runtime.publish_initial(&mut game, &mut candidate);
    (runtime, game, candidate)
}
pub(crate) fn digest(runtime: &GameRuntime<Rules>, game: &Rules) -> u64 {
    runtime
        .world_digest(&game.resource, |_, _| {})
        .expect("portable components")
}
fn main() {
    for (name, next) in [("infinity", Number::ONE / Number::ZERO), ("nan", Number::ZERO / Number::ZERO), ("finite", Number::ONE)] {
        let (mut runtime, mut game, mut candidate) = setup(1);
        let before = digest(&runtime, &game);
        game.next = next;
        let result = runtime.step(&mut game, &mut candidate);
        println!("{name}: step={result:?} tick={} resource_finite={} digest_changed={}", runtime.tick(), game.resource.is_finite(), digest(&runtime, &game) != before);
        assert!(result.is_ok());
        assert_eq!(runtime.tick(), 1);
        assert_eq!(game.resource.is_finite(), name == "finite");
    }
}
