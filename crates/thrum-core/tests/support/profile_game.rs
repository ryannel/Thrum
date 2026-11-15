use thrum_core::{
    Handle, StableId, WorldEpoch,
    profile::{Digest, Number, StateEncode},
    sim::{
        Access, AccessKey, AccessMode, ComponentRegistry, ComponentSchemaId, Edit, EditWriter,
        Executor, GameRuntime, RegisteredSystem, RegisteredWorldError, Stage, SystemId, SystemSpec,
        TickGame, WorldRead, WritePermit,
    },
};

#[derive(Clone, Copy)]
pub(crate) struct Scalar(Number);
impl StateEncode for Scalar {
    fn encode(&self, digest: &mut Digest) {
        self.0.encode(digest);
    }
}

pub(crate) struct Rules {
    pub write: WritePermit<Scalar>,
    pub handle: Handle,
    pub next: Number,
    pub resource: u64,
}
impl TickGame for Rules {
    type Error = RegisteredWorldError;
    type Candidate = Scalar;
    type ParallelOutput = ();
    fn reset_candidate(&self, candidate: &mut Scalar) {
        *candidate = Scalar(self.next);
    }
    fn reset_parallel_output(&self, (): &mut ()) {}
    fn merge_parallel(
        &self,
        _: SystemId,
        (): &mut (),
        _: &mut Scalar,
        _: u64,
        _: &mut EditWriter<'_, Scalar>,
    ) {
    }
    fn apply(&mut self, _: &WorldRead<'_>, _: u64, _: &Scalar) {
        self.resource += 1;
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
    _: &mut Scalar,
    edits: &mut EditWriter<'_, Scalar>,
) -> Result<(), RegisteredWorldError> {
    edits.push(Edit::replace(&game.write, game.handle, 0, |value, _| value));
    Ok(())
}
pub(crate) fn setup(epoch: u128) -> (GameRuntime<Rules>, Rules, Scalar) {
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
        resource: 0,
    };
    let mut candidate = Scalar(Number::ZERO);
    runtime.publish_initial(&mut game, &mut candidate);
    (runtime, game, candidate)
}
pub(crate) fn digest(runtime: &GameRuntime<Rules>, game: &Rules) -> u64 {
    runtime
        .world_digest(&game.resource, |_, _| {})
        .expect("portable components")
}
pub(crate) fn number_digest(number: Number) -> u64 {
    let mut digest = Digest::default();
    number.encode(&mut digest);
    digest.finish()
}
