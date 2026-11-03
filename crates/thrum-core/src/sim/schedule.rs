/// Authoritative stage order. Every system in one stage finishes before the next.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    /// Map admitted controls and one-shot actions into tick work.
    Input,
    /// Advance physics, AI and timers even when no new action arrived.
    Simulate,
    /// Resolve ordered edits and validate the complete candidate tick.
    Resolve,
    /// Commit the validated tick; consumers can only observe this endpoint.
    Publish,
}

/// Stable registration identity within one compiled schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SystemId(pub u32);

/// Components in the provisional A1 fixture. This is not a reflection schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentKind {
    /// World-space position in metres.
    Position,
    /// Velocity in metres/second.
    Velocity,
    /// Fixture health.
    Health,
    /// Peer/actor control identity.
    Ownership,
    /// Opaque visual selection, consumed only after commit.
    Visual,
}

/// Shared access identity. External IDs must name one owner-defined resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessKey {
    /// One W1 component column.
    Component(ComponentKind),
    /// A named external resource, distinct from any component.
    External(u64),
}

/// Declared system access; a write conflicts with any other access to its key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    /// Read-only access.
    Read,
    /// Exclusive access.
    Write,
}

/// A system's declared component or external-resource access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Access {
    /// Shared identity of the accessed state.
    pub key: AccessKey,
    /// Read or exclusive write.
    pub mode: AccessMode,
}

/// Setup-time system declaration. No closures or backend storage escape here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemSpec {
    /// Unique identity in this schedule.
    pub id: SystemId,
    /// Mandatory authoritative stage.
    pub stage: Stage,
    /// Component/resource reads and writes.
    pub accesses: Vec<Access>,
    /// Explicit predecessors. They must not come from a later stage.
    pub after: Vec<SystemId>,
}

/// Graph construction error; a failed compile produces no executable schedule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleError {
    /// Registration identities must be unique.
    DuplicateSystem,
    /// Explicit predecessor is absent.
    UnknownDependency,
    /// An explicit edge contradicts input → simulate → resolve → publish.
    BackwardStage,
    /// Explicit edges or conflict ordering form a cycle.
    Cycle,
}

/// Validated deterministic schedule. Setup allocates O(systems²) edges and
/// retains conflict-free dependency batches; serial execution walks a flat
/// order without allocation. Same-stage access conflicts follow registration
/// order; explicit reverse edges therefore reject. Declarations are a
/// scheduler contract, not dynamic borrow enforcement.
#[derive(Clone, Debug)]
pub struct Schedule {
    systems: Vec<SystemSpec>,
    order: Vec<usize>,
    batches: Vec<Vec<SystemId>>,
    edges: Vec<(SystemId, SystemId)>,
}

impl Schedule {
    /// Compile stage, explicit-dependency and conflicting-access edges once.
    pub fn new(systems: Vec<SystemSpec>) -> Result<Self, ScheduleError> {
        let n = systems.len();
        for (i, system) in systems.iter().enumerate() {
            if systems[..i].iter().any(|prior| prior.id == system.id) {
                return Err(ScheduleError::DuplicateSystem);
            }
        }
        let mut matrix = vec![false; n.checked_mul(n).ok_or(ScheduleError::Cycle)?];
        for (i, a) in systems.iter().enumerate() {
            for (j, b) in systems.iter().enumerate() {
                if a.stage < b.stage || (i < j && a.stage == b.stage && conflicts(a, b)) {
                    matrix[i * n + j] = true;
                }
            }
            for dependency in &a.after {
                let j = systems
                    .iter()
                    .position(|s| s.id == *dependency)
                    .ok_or(ScheduleError::UnknownDependency)?;
                if systems[j].stage > a.stage {
                    return Err(ScheduleError::BackwardStage);
                }
                matrix[j * n + i] = true;
            }
        }
        let mut indegree = vec![0usize; n];
        let mut edges = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if matrix[i * n + j] {
                    indegree[j] += 1;
                    edges.push((systems[i].id, systems[j].id));
                }
            }
        }
        let mut order = Vec::with_capacity(n);
        let mut batches = Vec::new();
        let mut visited = vec![false; n];
        while order.len() < n {
            let ready: Vec<usize> = (0..n)
                .filter(|&i| !visited[i] && indegree[i] == 0)
                .collect();
            if ready.is_empty() {
                return Err(ScheduleError::Cycle);
            }
            batches.push(ready.iter().map(|&i| systems[i].id).collect());
            for &i in &ready {
                visited[i] = true;
                order.push(i);
            }
            for &i in &ready {
                for j in 0..n {
                    if matrix[i * n + j] {
                        indegree[j] -= 1;
                    }
                }
            }
        }
        Ok(Self {
            systems,
            order,
            batches,
            edges,
        })
    }

    /// Compiled execution order, stable for the same declarations.
    pub fn order(&self) -> impl ExactSizeIterator<Item = &SystemSpec> {
        self.order.iter().map(|&i| &self.systems[i])
    }

    /// Deterministic dependency waves. IDs within a wave have no declared
    /// conflicts or dependency edge, but A1 executes them serially.
    pub fn batches(&self) -> impl ExactSizeIterator<Item = &[SystemId]> {
        self.batches.iter().map(Vec::as_slice)
    }

    /// Retained dependency edges for graph inspection and a future executor.
    pub fn edges(&self) -> &[(SystemId, SystemId)] {
        &self.edges
    }

    /// Run the same graph serially. Failure prevents all later systems, including
    /// publication. The runner owns staged writes and its recovery/fault policy.
    pub fn run<E>(&self, mut run: impl FnMut(&SystemSpec) -> Result<(), E>) -> Result<(), E> {
        self.order().try_for_each(&mut run)
    }
}

fn conflicts(a: &SystemSpec, b: &SystemSpec) -> bool {
    a.accesses.iter().any(|a| {
        b.accesses
            .iter()
            .any(|b| a.key == b.key && (a.mode == AccessMode::Write || b.mode == AccessMode::Write))
    })
}
