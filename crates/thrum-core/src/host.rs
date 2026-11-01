//! Native/headless callback seam for the bounded I1 authority and replica.
//! A platform event loop calls `on_callback` with monotonic `Instant` values.

use std::{
    collections::BTreeSet,
    panic::{AssertUnwindSafe, catch_unwind},
    time::{Duration, Instant},
};

use crate::{
    Admission, AdmissionError, AdmittedGroup, Authority, ClockConfig, ClockError, ClockReport,
    ClockState, GameRules, Input, PeerId, Replica, ReplicaError, Snapshot, StepError, TickClock,
    TickPublication, WorldWork,
};

/// CPU admission policy for one host callback. A cost bound is an input from
/// measurement and reserve planning, not a guarantee about actual execution.
#[derive(Clone, Copy, Debug)]
pub struct HostBudget {
    /// CPU time available before this host callback's deadline.
    pub callback_cpu: Duration,
    /// Time reserved for other callback work outside world steps.
    pub other_work: Duration,
    /// Conservative cost admitted for one complete world/replica step.
    pub tick_cost_bound: Duration,
}

impl HostBudget {
    fn valid(self) -> bool {
        !self.callback_cpu.is_zero()
            && !self.tick_cost_bound.is_zero()
            && self.other_work < self.callback_cpu
    }

    fn admitted_ticks(self) -> u32 {
        let count = self.available_step_cpu().as_nanos() / self.tick_cost_bound.as_nanos();
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    fn available_step_cpu(self) -> Duration {
        self.callback_cpu - self.other_work
    }
}

/// Input cannot enter an inactive or faulted host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostAdmissionError {
    /// The host is paused, overloaded or terminally faulted.
    Unavailable,
    /// Authority rejected the input envelope.
    Authority(AdmissionError),
}

/// Host callback or setup failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostError {
    /// CPU admission parameters cannot service a tick.
    InvalidBudget,
    /// Host supplied a timestamp before its origin or the clock rejected it.
    Clock(ClockError),
    /// The authority encountered a terminal game-contract fault. The host
    /// cannot expose the partial tick or retry it.
    Authority(StepError),
    /// The local replica rejected a committed authority group; this host is
    /// terminally faulted and may not replay that tick.
    Replica(ReplicaError),
    /// A result acknowledgement was inconsistent with the committed prefix.
    Acknowledgement(AdmissionError),
    /// A previous post-commit fault already stopped this host.
    Faulted,
}

/// One committed tick delivered to the host caller. The local path retains
/// its complete group; the in-memory replica path returns admitted outcomes
/// and the committed snapshot. The caller decides when to acknowledge them.
pub enum HostPublication<T, E, R> {
    /// Authority-only publication, including full state changes and results.
    Local(TickPublication<T, E, R>),
    /// State admitted by the replica, with results/events still observable.
    Replicated {
        /// Immutable authority state at this tick.
        snapshot: Snapshot<T>,
        /// Atomically admitted outcomes and events.
        admitted: AdmittedGroup<E, R>,
        /// Counted world publication work.
        world_work: WorldWork,
    },
}

/// Observed work and delivered publications for one real host callback.
pub struct HostCallback<T, E, R> {
    /// Fixed-step admission, committed work and pending/overload state.
    pub clock: ClockReport<StepError>,
    /// Terminal fault encountered after earlier ticks committed in this
    /// callback. Those earlier publications remain deliverable exactly once.
    pub terminal_fault: Option<HostError>,
    /// Measured wall time spent in this callback, including stage work.
    pub observed_elapsed: Duration,
    /// World-step work exceeded `callback_cpu - other_work` and entered overload.
    pub step_budget_overrun: bool,
    /// Counted world copy work over successfully committed ticks.
    pub world_work: WorldWork,
    /// Full groups admitted by the in-memory replica.
    pub replica_groups: u32,
    /// Committed ticks and their observable outputs, in tick order.
    pub publications: Vec<HostPublication<T, E, R>>,
}

/// Initial admitted outcomes when a game host starts its replica.
pub type InitialOutcomes<G> = AdmittedGroup<<G as GameRules>::Event, <G as GameRules>::Rejection>;

/// Callback output for one concrete game rule set.
pub type GameHostCallback<G> =
    HostCallback<<G as GameRules>::Entity, <G as GameRules>::Event, <G as GameRules>::Rejection>;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Stage {
    AuthorityStep,
    ReplicaAdmission,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Phase {
    Commit,
    Observe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Role {
    Authority,
    Replica,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Resource {
    World,
    Group,
    Replica,
}

struct StageSpec {
    stage: Stage,
    phase: Phase,
    role: Role,
    reads: &'static [Resource],
    writes: &'static [Resource],
    after: &'static [Stage],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScheduleError {
    Duplicate,
    MissingDependency,
    BackwardDependency,
    WrongOwner,
    UnorderedConflict,
}

/// Validate access once, then retain only the prepared stage order on the hot
/// path. The finite I1 registration uses typed resources rather than an
/// unqualified dynamic schema or a speculative general scheduler.
fn prepare_schedule(specs: &[StageSpec]) -> Result<Vec<Stage>, ScheduleError> {
    let mut seen = BTreeSet::new();
    for spec in specs {
        if !seen.insert(spec.stage) {
            return Err(ScheduleError::Duplicate);
        }
        if spec.writes.iter().any(|resource| {
            matches!(
                (spec.role, resource),
                (Role::Authority, Resource::Replica)
                    | (Role::Replica, Resource::World | Resource::Group)
            )
        }) || spec.reads.iter().any(|resource| {
            matches!(
                (spec.role, resource),
                (Role::Authority, Resource::Replica) | (Role::Replica, Resource::World)
            )
        }) {
            return Err(ScheduleError::WrongOwner);
        }
    }
    for spec in specs {
        for dependency in spec.after {
            let Some(prior) = specs
                .iter()
                .find(|candidate| candidate.stage == *dependency)
            else {
                return Err(ScheduleError::MissingDependency);
            };
            if prior.phase > spec.phase || prior.stage == spec.stage {
                return Err(ScheduleError::BackwardDependency);
            }
        }
    }
    for (index, left) in specs.iter().enumerate() {
        for right in &specs[index + 1..] {
            if left.phase != right.phase {
                continue;
            }
            let conflict =
                left.writes.iter().any(|resource| {
                    right.writes.contains(resource) || right.reads.contains(resource)
                }) || right
                    .writes
                    .iter()
                    .any(|resource| left.reads.contains(resource));
            if conflict
                && !ordered_by_dependency(specs, left.stage, right.stage)
                && !ordered_by_dependency(specs, right.stage, left.stage)
            {
                return Err(ScheduleError::UnorderedConflict);
            }
        }
    }
    let mut order = Vec::with_capacity(specs.len());
    while order.len() < specs.len() {
        let next = specs
            .iter()
            .filter(|spec| !order.contains(&spec.stage))
            .filter(|spec| {
                spec.after
                    .iter()
                    .all(|dependency| order.contains(dependency))
            })
            .filter(|spec| {
                !specs
                    .iter()
                    .any(|other| other.phase < spec.phase && !order.contains(&other.stage))
            })
            .min_by_key(|spec| (spec.phase, spec.stage));
        let Some(next) = next else {
            return Err(ScheduleError::BackwardDependency);
        };
        order.push(next.stage);
    }
    Ok(order)
}

fn ordered_by_dependency(specs: &[StageSpec], first: Stage, later: Stage) -> bool {
    let mut pending = vec![later];
    let mut visited = BTreeSet::new();
    while let Some(stage) = pending.pop() {
        if !visited.insert(stage) {
            continue;
        }
        let Some(spec) = specs.iter().find(|spec| spec.stage == stage) else {
            continue;
        };
        for dependency in spec.after {
            if *dependency == first {
                return true;
            }
            pending.push(*dependency);
        }
    }
    false
}

fn headless_schedule(with_replica: bool) -> Vec<Stage> {
    let mut specs = vec![StageSpec {
        stage: Stage::AuthorityStep,
        phase: Phase::Commit,
        role: Role::Authority,
        reads: &[],
        writes: &[Resource::World, Resource::Group],
        after: &[],
    }];
    if with_replica {
        specs.push(StageSpec {
            stage: Stage::ReplicaAdmission,
            phase: Phase::Observe,
            role: Role::Replica,
            reads: &[Resource::Group],
            writes: &[Resource::Replica],
            after: &[Stage::AuthorityStep],
        });
    }
    prepare_schedule(&specs).expect("built-in I1 stage declarations must validate")
}

/// One bounded headless local or in-memory match host. It owns the authority
/// and optionally its confirmed replica; no second rule implementation exists.
pub struct HeadlessHost<G: GameRules> {
    origin: Instant,
    clock: TickClock,
    authority: Authority<G>,
    replica: Option<Replica<G::Entity>>,
    schedule: Vec<Stage>,
    budget: HostBudget,
    admitted_ticks_per_callback: u32,
    faulted: bool,
}

impl<G: GameRules> HeadlessHost<G> {
    /// Construct a local headless host at the monotonic start instant.
    pub fn local(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        authority: Authority<G>,
    ) -> Result<Self, HostError> {
        Self::build(origin, clock, budget, authority, None)
    }

    /// Construct a match host with a complete initial in-memory replica.
    pub fn with_replica(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        authority: Authority<G>,
    ) -> Result<(Self, InitialOutcomes<G>), HostError> {
        if authority.is_faulted() {
            return Err(HostError::Authority(StepError::TerminalGameContract));
        }
        let (replica, initial_outcomes) =
            Replica::from_baseline(authority.try_baseline().map_err(HostError::Authority)?)
                .map_err(HostError::Replica)?;
        let host = Self::build(origin, clock, budget, authority, Some(replica))?;
        Ok((host, initial_outcomes))
    }

    fn build(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        authority: Authority<G>,
        replica: Option<Replica<G::Entity>>,
    ) -> Result<Self, HostError> {
        if authority.is_faulted() {
            return Err(HostError::Authority(StepError::TerminalGameContract));
        }
        if !budget.valid() || budget.admitted_ticks() == 0 {
            return Err(HostError::InvalidBudget);
        }
        let admitted_ticks_per_callback = budget.admitted_ticks();
        let schedule = headless_schedule(replica.is_some());
        Ok(Self {
            origin,
            clock: TickClock::new(clock, Duration::ZERO).map_err(HostError::Clock)?,
            authority,
            replica,
            schedule,
            budget,
            admitted_ticks_per_callback,
            faulted: false,
        })
    }

    /// Read the committed authority for inspection or baseline creation.
    pub fn authority(&self) -> &Authority<G> {
        &self.authority
    }

    /// Read confirmed state when this host has an in-memory replica.
    pub fn replica(&self) -> Option<&Replica<G::Entity>> {
        self.replica.as_ref()
    }

    /// Current local clock state.
    pub fn clock_state(&self) -> ClockState {
        self.clock.state()
    }

    /// Admit input only while this host is active.
    pub fn submit(
        &mut self,
        input: Input<G::Action>,
    ) -> Result<Admission<G::Rejection>, HostAdmissionError> {
        if self.faulted || self.authority.is_faulted() || self.clock.state() != ClockState::Active {
            return Err(HostAdmissionError::Unavailable);
        }
        self.authority
            .submit(input)
            .map_err(HostAdmissionError::Authority)
    }

    /// Retire results only after the caller has received and handled their
    /// group. This works for both local and replicated host paths.
    pub fn acknowledge_results(&mut self, peer: PeerId, through: u64) -> Result<(), HostError> {
        if self.faulted || self.authority.is_faulted() {
            return Err(HostError::Faulted);
        }
        self.authority
            .acknowledge_results(peer, through)
            .map_err(HostError::Acknowledgement)
    }

    /// Stop accumulating local world time. Already admitted work may drain.
    pub fn pause(&mut self) {
        self.clock.pause();
    }

    /// Resume after a pause or overload from a new monotonic origin.
    pub fn resume(&mut self, now: Instant) -> Result<(), HostError> {
        if self.faulted || self.authority.is_faulted() {
            return Err(HostError::Faulted);
        }
        let elapsed = now
            .checked_duration_since(self.origin)
            .ok_or(HostError::Clock(ClockError::NonMonotonicTime))?;
        self.clock.resume(elapsed).map_err(HostError::Clock)
    }

    /// Service one actual platform callback. A callback that exceeds the CPU
    /// budget visibly overloads this local host after its committed ticks;
    /// it cannot retroactively preempt a slow game rule.
    pub fn on_callback(&mut self, now: Instant) -> Result<GameHostCallback<G>, HostError> {
        if self.faulted || self.authority.is_faulted() {
            return Err(HostError::Faulted);
        }
        let elapsed = now
            .checked_duration_since(self.origin)
            .ok_or(HostError::Clock(ClockError::NonMonotonicTime))?;
        let started = Instant::now();
        let mut work = WorldWork::default();
        let mut replica_groups = 0;
        let mut publications = Vec::new();
        let mut fatal = None;
        let authority = &mut self.authority;
        let replica = &mut self.replica;
        let schedule = &self.schedule;
        let step_cpu_budget = self.budget.available_step_cpu();
        let mut report = self
            .clock
            .advance_controlled(elapsed, self.admitted_ticks_per_callback, || {
                let mut publication = None;
                for stage in schedule {
                    match stage {
                        Stage::AuthorityStep => {
                            let committed = catch_unwind(AssertUnwindSafe(|| authority.step()))
                                .map_err(|_| StepError::TerminalGameContract)??;
                            work.page_references += committed.world_work.page_references;
                            work.copied_pages += committed.world_work.copied_pages;
                            work.copied_objects += committed.world_work.copied_objects;
                            work.group_objects += committed.world_work.group_objects;
                            publication = Some(committed);
                        }
                        Stage::ReplicaAdmission => {
                            let committed = publication.take().expect("validated producer order");
                            let Some(replica) = replica else {
                                unreachable!("replica stage requires a replica")
                            };
                            match replica.apply(committed.group) {
                                Ok(admitted) => {
                                    replica_groups += 1;
                                    publications.push(HostPublication::Replicated {
                                        snapshot: committed.snapshot,
                                        admitted,
                                        world_work: committed.world_work,
                                    });
                                }
                                Err(error) => fatal = Some(HostError::Replica(error)),
                            }
                        }
                    }
                    if fatal.is_some() {
                        break;
                    }
                }
                if let Some(committed) = publication {
                    publications.push(HostPublication::Local(committed));
                }
                // A bound based on prior measurement is only an admission
                // estimate. Stop catch-up after an actual slow committed tick.
                Ok(fatal.is_none() && started.elapsed() < step_cpu_budget)
            })
            .map_err(HostError::Clock)?;
        let observed_elapsed = started.elapsed();
        let terminal_fault = if report.step_error == Some(StepError::TerminalGameContract) {
            Some(HostError::Authority(StepError::TerminalGameContract))
        } else {
            fatal
        };
        if let Some(fault) = terminal_fault {
            self.faulted = true;
            if publications.is_empty() {
                return Err(fault);
            }
        }
        let step_budget_overrun = observed_elapsed > step_cpu_budget;
        if step_budget_overrun && terminal_fault.is_none() {
            self.clock.report_cpu_overload();
            report.state = ClockState::Overloaded;
        }
        Ok(HostCallback {
            clock: report,
            terminal_fault,
            observed_elapsed,
            step_budget_overrun,
            world_work: work,
            replica_groups,
            publications,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_and_observation_share_the_reserved_step_budget() {
        let budget = HostBudget {
            callback_cpu: Duration::from_millis(10),
            other_work: Duration::from_millis(8),
            tick_cost_bound: Duration::from_millis(1),
        };
        assert_eq!(budget.available_step_cpu(), Duration::from_millis(2));
        assert_eq!(budget.admitted_ticks(), 2);
    }

    #[test]
    fn schedule_rejects_unordered_conflict_and_wrong_owner() {
        let conflict = [
            StageSpec {
                stage: Stage::AuthorityStep,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[Resource::World],
                after: &[],
            },
            StageSpec {
                stage: Stage::ReplicaAdmission,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[Resource::World],
                writes: &[],
                after: &[],
            },
        ];
        assert_eq!(
            prepare_schedule(&conflict).err(),
            Some(ScheduleError::UnorderedConflict)
        );
        let wrong_owner = [StageSpec {
            stage: Stage::ReplicaAdmission,
            phase: Phase::Observe,
            role: Role::Replica,
            reads: &[],
            writes: &[Resource::World],
            after: &[],
        }];
        assert_eq!(
            prepare_schedule(&wrong_owner).err(),
            Some(ScheduleError::WrongOwner)
        );
        let forbidden_read = [StageSpec {
            stage: Stage::ReplicaAdmission,
            phase: Phase::Observe,
            role: Role::Replica,
            reads: &[Resource::World],
            writes: &[],
            after: &[],
        }];
        assert_eq!(
            prepare_schedule(&forbidden_read).err(),
            Some(ScheduleError::WrongOwner)
        );
    }

    #[test]
    fn schedule_rejects_missing_and_backward_dependencies() {
        let missing = [StageSpec {
            stage: Stage::ReplicaAdmission,
            phase: Phase::Observe,
            role: Role::Replica,
            reads: &[],
            writes: &[Resource::Replica],
            after: &[Stage::AuthorityStep],
        }];
        assert_eq!(
            prepare_schedule(&missing).err(),
            Some(ScheduleError::MissingDependency)
        );
        let backwards = [
            StageSpec {
                stage: Stage::AuthorityStep,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[Resource::World],
                after: &[Stage::ReplicaAdmission],
            },
            StageSpec {
                stage: Stage::ReplicaAdmission,
                phase: Phase::Observe,
                role: Role::Replica,
                reads: &[],
                writes: &[Resource::Replica],
                after: &[],
            },
        ];
        assert_eq!(
            prepare_schedule(&backwards).err(),
            Some(ScheduleError::BackwardDependency)
        );
        let cycle = [
            StageSpec {
                stage: Stage::AuthorityStep,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[Resource::World],
                after: &[Stage::ReplicaAdmission],
            },
            StageSpec {
                stage: Stage::ReplicaAdmission,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[Resource::Group],
                after: &[Stage::AuthorityStep],
            },
        ];
        assert_eq!(
            prepare_schedule(&cycle).err(),
            Some(ScheduleError::BackwardDependency)
        );
    }
}
