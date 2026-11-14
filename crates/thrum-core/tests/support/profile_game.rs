use thrum_core::{
    Handle, StableId, WorldEpoch,
    profile::{Digest, Number, StateEncode},
    sim::{
        Access, AccessKey, AccessMode, ComponentRegistry, ComponentSchemaId, Edit, EditWriter,
        Executor, GameRuntime, RegisteredSystem, RegisteredWorldError, Stage, SystemId, SystemSpec,
        TickGame, WorldRead, WritePermit,
    },
};

pub(crate) struct Rules {
    pub write: WritePermit<Number>,
    pub handle: Handle,
    pub next: Number,
    pub resource: u64,
}
impl TickGame<()> for Rules {
    type Error = RegisteredWorldError;
    type Candidate = Number;
    type ParallelOutput = ();
    fn reset_candidate(&self, candidate: &mut Number) {
        *candidate = self.next;
    }
    fn reset_parallel_output(&self, (): &mut ()) {}
    fn merge_parallel(
        &self,
        _: SystemId,
        (): &mut (),
        _: &mut Number,
        _: u64,
        _: &mut EditWriter<'_, Number>,
    ) {
    }
    fn apply(&mut self, _: &WorldRead<'_>, _: u64, _: &Number) {
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
    _: &mut Number,
    edits: &mut EditWriter<'_, Number>,
) -> Result<(), RegisteredWorldError> {
    edits.push(Edit::replace(&game.write, game.handle, 0, |value, _| value));
    Ok(())
}
pub(crate) fn setup(epoch: u128) -> (GameRuntime<(), Rules>, Rules, Number) {
    let mut registry = ComponentRegistry::default();
    let key = registry
        .register_profile::<Number>(ComponentSchemaId(123))
        .expect("schema");
    let mut runtime = GameRuntime::new(
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
        || (),
    )
    .expect("runtime");
    let handle = runtime
        .seed(StableId(987), |writer| {
            writer.add(Number::ONE)?;
            Ok(())
        })
        .expect("seed");
    let mut game = Rules {
        write: runtime.write_permit(SystemId(1)).expect("permit"),
        handle,
        next: Number::ONE,
        resource: 0,
    };
    let mut candidate = Number::ZERO;
    runtime.publish_initial(&mut game, &mut candidate);
    (runtime, game, candidate)
}
pub(crate) fn digest(runtime: &GameRuntime<(), Rules>, game: &Rules) -> u64 {
    runtime
        .world_digest(&game.resource, |_, _| {})
        .expect("portable components")
}
pub(crate) fn number_digest(number: Number) -> u64 {
    let mut digest = Digest::default();
    number.encode(&mut digest);
    digest.finish()
}
