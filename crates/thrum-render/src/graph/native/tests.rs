use super::*;
use crate::graph::{Access, PassKind, PassScope, ResourceDescriptor};
use std::collections::BTreeSet;

fn access(resource: ResourceId, kind: AccessKind) -> Access {
    Access {
        resource,
        range: 0..8,
        kind,
        store: true,
        view: None,
    }
}

fn pass(name: &str, accesses: Vec<Access>) -> Pass {
    Pass {
        name: name.into(),
        scope: PassScope::Shared,
        pipeline: None,
        accesses,
        external_side_effect: true,
    }
}

fn resource(graph: &mut FrameGraph, scope: ResourceScope) -> ResourceId {
    graph
        .resource(
            ResourceDescriptor::Buffer {
                bytes: 8,
                usage: wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::STORAGE,
            },
            scope,
        )
        .expect("native fixture resource")
}

#[test]
fn native_writes_require_initialized_retained_lifetime() {
    for scope in [
        ResourceScope::Transient,
        ResourceScope::Imported { initialized: false },
        ResourceScope::History {
            owner: 1,
            epoch: 1,
            initialized: false,
        },
    ] {
        let mut graph = FrameGraph::default();
        let target = resource(&mut graph, scope);
        graph.native_pass(pass(
            "native",
            vec![access(target, AccessKind::StorageWrite)],
        ));
        assert_eq!(
            graph.compile(1, &BTreeSet::new()).err(),
            Some(GraphError::NativeResource { pass: 0 })
        );
    }
}

#[test]
fn native_hazards_order_wgpu_producer_native_segment_and_consumer() {
    let mut graph = FrameGraph::default();
    let source = resource(&mut graph, ResourceScope::Transient);
    let output = resource(
        &mut graph,
        ResourceScope::Persistent {
            owner: 1,
            epoch: 1,
            initialized: true,
        },
    );
    graph.pass(pass(
        "wgpu producer",
        vec![access(source.clone(), AccessKind::CopyWrite)],
    ));
    graph.native_pass(pass(
        "native",
        vec![
            access(source, AccessKind::CopyRead),
            access(output.clone(), AccessKind::StorageWrite),
        ],
    ));
    graph.pass(pass(
        "wgpu consumer",
        vec![access(output.clone(), AccessKind::CopyRead)],
    ));
    let plan = graph.compile(1, &BTreeSet::new()).expect("native graph");
    assert_eq!(
        plan.pass_names().collect::<Vec<_>>(),
        ["wgpu producer", "native", "wgpu consumer"]
    );
    assert!(plan.dependencies.contains(&(0, 1)));
    assert!(plan.dependencies.contains(&(1, 2)));
    assert!(
        plan.lifetimes
            .iter()
            .find(|life| life.resource == output)
            .expect("retained output")
            .transient_slot
            .is_none()
    );
    assert_eq!(
        plan.pass_bindings().nth(1).expect("native pass").1.kind,
        Some(PassKind::Native)
    );
    assert_eq!(
        graph
            .compile_late_surface(1, &BTreeSet::new(), &output, &BTreeSet::from([2]))
            .err(),
        Some(GraphError::NativeSurface { pass: 1 })
    );
}
