//! Validation of the provisional A1 serial schedule and its declared access graph.

use thrum_core::sim::{
    Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId,
    RegistrationError, Schedule, ScheduleError, Stage, SystemId, SystemSpec,
};

struct TestPosition;
struct TestVelocity;

fn schedule(systems: Vec<SystemSpec>) -> Result<Schedule, ScheduleError> {
    let mut registry = ComponentRegistry::default();
    registry
        .register::<TestPosition>(ComponentSchemaId(1))
        .expect("test component");
    registry
        .register::<TestVelocity>(ComponentSchemaId(2))
        .expect("test component");
    Schedule::new(&registry, systems)
}

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
        key: AccessKey::Component(ComponentKey::of::<TestPosition>()),
        mode,
    }
}

#[test]
fn registration_keeps_game_component_accesses_distinct_and_refuses_unknown_types() {
    struct Unregistered;
    let mut registry = ComponentRegistry::default();
    let position = registry
        .register::<TestPosition>(ComponentSchemaId(1))
        .expect("position registers");
    assert_eq!(
        registry.register::<TestPosition>(ComponentSchemaId(2)),
        Err(RegistrationError::DuplicateType)
    );
    assert_eq!(
        registry.register::<TestVelocity>(ComponentSchemaId(1)),
        Err(RegistrationError::DuplicateSchema)
    );
    let velocity = registry
        .register::<TestVelocity>(ComponentSchemaId(2))
        .expect("rejected schema did not mutate registry");
    assert_eq!(registry.schema(position), Some(ComponentSchemaId(1)));
    assert_eq!(registry.schema(velocity), Some(ComponentSchemaId(2)));

    let write = |key| Access {
        key: AccessKey::Component(key),
        mode: AccessMode::Write,
    };
    assert_eq!(
        Schedule::new(
            &registry,
            vec![spec(
                1,
                Stage::Simulate,
                vec![write(ComponentKey::of::<Unregistered>())],
                &[]
            )]
        )
        .unwrap_err(),
        ScheduleError::UnknownComponent
    );
    let graph = Schedule::new(
        &registry,
        vec![
            spec(1, Stage::Simulate, vec![write(position)], &[]),
            spec(2, Stage::Simulate, vec![write(velocity)], &[]),
        ],
    )
    .expect("distinct game components permit one wave");
    assert_eq!(
        graph.batches().next(),
        Some(&[SystemId(1), SystemId(2)][..])
    );
}

#[test]
fn stage_order_wins_registration_order_and_conflicts_add_dependency_edges() {
    let graph = schedule(vec![
        spec(4, Stage::Publish, vec![], &[]),
        spec(2, Stage::Simulate, vec![access(AccessMode::Read)], &[]),
        spec(1, Stage::Input, vec![], &[]),
        spec(3, Stage::Simulate, vec![access(AccessMode::Write)], &[]),
    ])
    .expect("test setup or assertion requires a value");
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
    let graph = schedule(vec![
        spec(1, Stage::Simulate, vec![], &[2]),
        spec(2, Stage::Simulate, vec![external], &[]),
        spec(3, Stage::Simulate, vec![external], &[]),
    ])
    .expect("test setup or assertion requires a value");
    assert_eq!(
        graph.order().map(|s| s.id).collect::<Vec<_>>(),
        [SystemId(2), SystemId(1), SystemId(3)]
    );
    assert!(graph.edges().contains(&(SystemId(2), SystemId(3))));
    assert_eq!(
        schedule(vec![spec(1, Stage::Input, vec![], &[2])]).unwrap_err(),
        ScheduleError::UnknownDependency
    );
    assert_eq!(
        schedule(vec![
            spec(1, Stage::Input, vec![], &[]),
            spec(1, Stage::Publish, vec![], &[])
        ])
        .unwrap_err(),
        ScheduleError::DuplicateSystem
    );
    assert_eq!(
        schedule(vec![
            spec(1, Stage::Input, vec![], &[2]),
            spec(2, Stage::Publish, vec![], &[])
        ])
        .unwrap_err(),
        ScheduleError::BackwardStage
    );
    assert_eq!(
        schedule(vec![
            spec(1, Stage::Resolve, vec![], &[2]),
            spec(2, Stage::Resolve, vec![], &[1])
        ])
        .unwrap_err(),
        ScheduleError::Cycle
    );
}

#[test]
fn read_only_access_does_not_conflict_and_failure_cannot_reach_publish() {
    let graph = schedule(vec![
        spec(1, Stage::Simulate, vec![access(AccessMode::Read)], &[2]),
        spec(2, Stage::Simulate, vec![access(AccessMode::Read)], &[]),
        spec(3, Stage::Resolve, vec![], &[]),
        spec(4, Stage::Publish, vec![], &[]),
    ])
    .expect("test setup or assertion requires a value");
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
    let graph = schedule(vec![
        spec(1, Stage::Input, vec![], &[]),
        spec(2, Stage::Simulate, vec![access(AccessMode::Read)], &[]),
        spec(3, Stage::Simulate, vec![access(AccessMode::Read)], &[]),
        spec(4, Stage::Simulate, vec![access(AccessMode::Write)], &[]),
        spec(5, Stage::Resolve, vec![], &[]),
        spec(6, Stage::Publish, vec![], &[]),
    ])
    .expect("test setup or assertion requires a value");
    assert_eq!(
        graph
            .batches()
            .map(<[SystemId]>::to_vec)
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
