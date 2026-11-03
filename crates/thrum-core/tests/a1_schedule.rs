//! Validation of the provisional A1 serial schedule and its declared access graph.

use thrum_core::sim::{
    Access, AccessKey, AccessMode, ComponentKind, Schedule, ScheduleError, Stage, SystemId,
    SystemSpec,
};

fn spec(id: u32, stage: Stage, accesses: Vec<Access>, after: &[u32]) -> SystemSpec {
    SystemSpec {
        id: SystemId(id),
        stage,
        accesses,
        after: after.iter().copied().map(SystemId).collect(),
    }
}

fn access(mode: AccessMode) -> Access {
    Access {
        key: AccessKey::Component(ComponentKind::Position),
        mode,
    }
}

#[test]
fn stage_order_wins_registration_order_and_conflicts_add_dependency_edges() {
    let graph = Schedule::new(vec![
        spec(4, Stage::Publish, vec![], &[]),
        spec(2, Stage::Simulate, vec![access(AccessMode::Read)], &[]),
        spec(1, Stage::Input, vec![], &[]),
        spec(3, Stage::Simulate, vec![access(AccessMode::Write)], &[]),
    ])
    .unwrap();
    assert_eq!(
        graph.order().map(|s| s.id).collect::<Vec<_>>(),
        [SystemId(1), SystemId(2), SystemId(3), SystemId(4)]
    );
    assert!(graph.edges().contains(&(SystemId(2), SystemId(3))));
    assert!(graph.edges().contains(&(SystemId(3), SystemId(4))));
    assert!(!graph.edges().contains(&(SystemId(3), SystemId(2))));
}

#[test]
fn explicit_dependencies_external_resources_and_graph_failures_are_validated() {
    let external = Access {
        key: AccessKey::External(1),
        mode: AccessMode::Write,
    };
    let graph = Schedule::new(vec![
        spec(1, Stage::Simulate, vec![], &[2]),
        spec(2, Stage::Simulate, vec![external], &[]),
        spec(3, Stage::Simulate, vec![external], &[]),
    ])
    .unwrap();
    assert_eq!(
        graph.order().map(|s| s.id).collect::<Vec<_>>(),
        [SystemId(2), SystemId(1), SystemId(3)]
    );
    assert!(graph.edges().contains(&(SystemId(2), SystemId(3))));
    assert_eq!(
        Schedule::new(vec![spec(1, Stage::Input, vec![], &[2])]).unwrap_err(),
        ScheduleError::UnknownDependency
    );
    assert_eq!(
        Schedule::new(vec![
            spec(1, Stage::Input, vec![], &[]),
            spec(1, Stage::Publish, vec![], &[])
        ])
        .unwrap_err(),
        ScheduleError::DuplicateSystem
    );
    assert_eq!(
        Schedule::new(vec![
            spec(1, Stage::Input, vec![], &[2]),
            spec(2, Stage::Publish, vec![], &[])
        ])
        .unwrap_err(),
        ScheduleError::BackwardStage
    );
    assert_eq!(
        Schedule::new(vec![
            spec(1, Stage::Resolve, vec![], &[2]),
            spec(2, Stage::Resolve, vec![], &[1])
        ])
        .unwrap_err(),
        ScheduleError::Cycle
    );
}

#[test]
fn read_only_access_does_not_conflict_and_failure_cannot_reach_publish() {
    let graph = Schedule::new(vec![
        spec(1, Stage::Simulate, vec![access(AccessMode::Read)], &[2]),
        spec(2, Stage::Simulate, vec![access(AccessMode::Read)], &[]),
        spec(3, Stage::Resolve, vec![], &[]),
        spec(4, Stage::Publish, vec![], &[]),
    ])
    .unwrap();
    let mut visited = Vec::new();
    assert_eq!(
        graph.run(|system| {
            visited.push(system.id);
            if system.stage == Stage::Resolve {
                Err("invalid candidate")
            } else {
                Ok(())
            }
        }),
        Err("invalid candidate")
    );
    assert_eq!(visited, [SystemId(2), SystemId(1), SystemId(3)]);
}

#[test]
fn batches_expose_only_conflict_free_ready_systems_with_stage_barriers() {
    let graph = Schedule::new(vec![
        spec(1, Stage::Input, vec![], &[]),
        spec(2, Stage::Simulate, vec![access(AccessMode::Read)], &[]),
        spec(3, Stage::Simulate, vec![access(AccessMode::Read)], &[]),
        spec(4, Stage::Simulate, vec![access(AccessMode::Write)], &[]),
        spec(5, Stage::Resolve, vec![], &[]),
        spec(6, Stage::Publish, vec![], &[]),
    ])
    .unwrap();
    assert_eq!(
        graph
            .batches()
            .map(|batch| batch.to_vec())
            .collect::<Vec<_>>(),
        vec![
            vec![SystemId(1)],
            vec![SystemId(2), SystemId(3)],
            vec![SystemId(4)],
            vec![SystemId(5)],
            vec![SystemId(6)]
        ]
    );
    assert_eq!(
        graph.order().map(|system| system.id).collect::<Vec<_>>(),
        [
            SystemId(1),
            SystemId(2),
            SystemId(3),
            SystemId(4),
            SystemId(5),
            SystemId(6)
        ]
    );
}
